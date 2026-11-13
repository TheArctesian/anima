//! SNMP check secrets against the database: sealed apart from `config`, bound to their check,
//! version, user name and device host, opened only for the probe of that host, and kept on
//! update only for the same version, user name and host (SEC-003, SEC-005, SEC-010, LOG-007,
//! D-5).

use anima_hub::{
    checks::{
        self, CheckChange, Definition, Error,
        config::{CheckConfig, SnmpConfig, SnmpCredentials, SnmpInput, SnmpSecrets, SnmpVersion},
    },
    credentials::Secret,
    ids::CheckId,
};
use sqlx::PgPool;

use super::{PVE_01, PVE_01_HOST, create, new_check, sealer, vault, vault_with};

macro_rules! devices_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const AUTH: &str = "auth-SECRET-face-1";
const PRIVACY: &str = "privacy-SECRET-face-2";
const COMMUNITY: &str = "community-SECRET-face-3";

fn config(version: SnmpVersion, username: Option<&str>) -> SnmpConfig {
    SnmpConfig {
        port: 161,
        version,
        oid: "1.3.6.1.2.1.1.5.0".to_owned(),
        expected: None,
        username: username.map(str::to_owned),
        timeout_ms: 5000,
    }
}

fn given(text: Option<&str>) -> Option<Secret> {
    text.map(|text| Secret::from(text.to_owned()))
}

fn v3(username: &str, auth: Option<&str>, privacy: Option<&str>) -> Definition {
    Definition::Snmp(SnmpInput {
        config: config(SnmpVersion::V3, Some(username)),
        credentials: SnmpCredentials {
            community: None,
            auth_password: given(auth),
            privacy_password: given(privacy),
        },
    })
}

fn v2c(community: Option<&str>) -> Definition {
    Definition::Snmp(SnmpInput {
        config: config(SnmpVersion::V2c, None),
        credentials: SnmpCredentials {
            community: given(community),
            ..SnmpCredentials::default()
        },
    })
}

fn text(secret: &Secret) -> String {
    String::from_utf8(secret.expose().to_vec()).unwrap()
}

async fn opened(pool: &PgPool, id: CheckId) -> Result<SnmpSecrets, Error> {
    let setup = checks::load_snmp_secrets(pool, &vault(), id, PVE_01_HOST)
        .await?
        .unwrap();
    Ok(setup.secrets.unwrap())
}

async fn passwords(pool: &PgPool, id: CheckId) -> (String, String) {
    match opened(pool, id).await.unwrap() {
        SnmpSecrets::User { auth, privacy } => (text(&auth), text(&privacy)),
        SnmpSecrets::Community(_) => panic!("a community"),
    }
}

async fn update(pool: &PgPool, id: CheckId, definition: Definition) -> Result<(), Error> {
    let mut conn = pool.acquire().await.unwrap();
    let change = CheckChange {
        definition: Some(definition),
        ..CheckChange::default()
    };
    checks::update_check(&mut conn, sealer(&vault()), id, change)
        .await
        .map(|_| ())
}

fn fields(result: Result<(), Error>) -> Vec<&'static str> {
    match result {
        Err(Error::Validation(errors)) => {
            for error in &errors {
                for planted in [AUTH, PRIVACY, COMMUNITY, "SECRET"] {
                    assert!(!error.message.contains(planted), "{error}");
                }
            }
            errors.iter().map(|error| error.field).collect()
        }
        other => panic!("not a validation error: {:?}", other.err()),
    }
}

async fn snmp_check(pool: &PgPool) -> CheckId {
    create(
        pool,
        new_check(PVE_01, "SNMP", v3("monitor", Some(AUTH), Some(PRIVACY))),
    )
    .await
}

devices_test! {
    async fn sec_003_snmp_secrets_are_sealed_and_appear_nowhere_in_the_row(pool: PgPool) {
        let id = snmp_check(&pool).await;
        let row: String = sqlx::query_scalar("SELECT row_to_json(c)::text FROM checks.checks c WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        for secret in [AUTH, PRIVACY, "SECRET"] {
            assert!(!row.contains(secret), "{secret} in the row");
        }
        let check = checks::get(&pool, id).await.unwrap();
        assert!(!format!("{check:?}").contains("SECRET"));
        let CheckConfig::Snmp(stored) = check.config else { panic!("not snmp") };
        assert_eq!(stored, config(SnmpVersion::V3, Some("monitor")));
        assert_eq!(passwords(&pool, id).await, (AUTH.to_owned(), PRIVACY.to_owned()));
    }

    async fn sec_003_snmp_secrets_open_only_for_their_own_check_and_key(pool: PgPool) {
        let id = snmp_check(&pool).await;
        let other = create(&pool, new_check(PVE_01, "Copy", v2c(Some(COMMUNITY)))).await;
        sqlx::query(
            "UPDATE checks.checks SET secrets_sealed = (SELECT secrets_sealed FROM checks.checks WHERE id = $1), \
             config = (SELECT config FROM checks.checks WHERE id = $1) WHERE id = $2",
        )
        .bind(id)
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(opened(&pool, other).await, Err(Error::Credentials(_))));
        let wrong_key = checks::load_snmp_secrets(&pool, &vault_with(8), id, PVE_01_HOST).await;
        assert!(matches!(wrong_key, Err(Error::Credentials(_))));
    }

    async fn sec_005_a_database_write_to_the_version_or_user_unbinds_the_secrets(pool: PgPool) {
        let id = snmp_check(&pool).await;
        for edit in [
            r#"jsonb_set(config, '{username}', '"other"')"#,
            r#"(config - 'username') || '{"version":"V2C"}'"#,
        ] {
            let kept: serde_json::Value = sqlx::query_scalar("SELECT config FROM checks.checks WHERE id = $1")
                .bind(id).fetch_one(&pool).await.unwrap();
            let sql = format!("UPDATE checks.checks SET config = {edit} WHERE id = $1");
            sqlx::query(&sql).bind(id).execute(&pool).await.unwrap();
            let loaded = checks::load_snmp_secrets(&pool, &vault(), id, PVE_01_HOST).await;
            assert!(matches!(loaded, Err(Error::SecretsUnbound)), "{edit}");
            sqlx::query("UPDATE checks.checks SET config = $2 WHERE id = $1")
                .bind(id).bind(kept).execute(&pool).await.unwrap();
            assert!(checks::load_snmp_secrets(&pool, &vault(), id, PVE_01_HOST).await.is_ok());
        }
    }

    async fn sec_005_an_omitted_secret_is_kept_for_the_same_version_and_user(pool: PgPool) {
        let id = snmp_check(&pool).await;
        update(&pool, id, v3("monitor", None, None)).await.unwrap();
        assert_eq!(passwords(&pool, id).await, (AUTH.to_owned(), PRIVACY.to_owned()));
        update(&pool, id, v3("monitor", Some("new-auth-password"), Some(""))).await.unwrap();
        assert_eq!(passwords(&pool, id).await, ("new-auth-password".to_owned(), PRIVACY.to_owned()));
    }

    async fn sec_005_a_changed_user_or_version_asks_for_the_secrets_and_changes_nothing(pool: PgPool) {
        let id = snmp_check(&pool).await;
        let renamed = update(&pool, id, v3("other", None, None)).await;
        assert_eq!(fields(renamed), ["authPassword"]);
        assert_eq!(fields(update(&pool, id, v2c(None)).await), ["community"]);
        assert_eq!(passwords(&pool, id).await, (AUTH.to_owned(), PRIVACY.to_owned()));
        update(&pool, id, v2c(Some(COMMUNITY))).await.unwrap();
        let SnmpSecrets::Community(community) = opened(&pool, id).await.unwrap() else {
            panic!("not a community");
        };
        assert_eq!(text(&community), COMMUNITY);
        assert_eq!(fields(update(&pool, id, v3("monitor", None, Some(PRIVACY))).await), ["authPassword"]);
    }

    async fn sec_005_secrets_that_no_longer_open_are_asked_for_again(pool: PgPool) {
        let id = snmp_check(&pool).await;
        sqlx::query("UPDATE checks.checks SET secrets_sealed = 'garbage' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        let left_out = update(&pool, id, v3("monitor", None, None)).await;
        assert_eq!(fields(left_out), ["config"]);
        update(&pool, id, v3("monitor", Some(AUTH), Some(PRIVACY))).await.unwrap();
        assert_eq!(passwords(&pool, id).await, (AUTH.to_owned(), PRIVACY.to_owned()));
    }

    async fn d_5_another_type_has_no_snmp_setup(pool: PgPool) {
        let ping = create(&pool, new_check(PVE_01, "Ping", super::icmp())).await;
        assert!(checks::load_snmp_secrets(&pool, &vault(), ping, PVE_01_HOST).await.unwrap().is_none());
        let missing = CheckId::from(uuid::Uuid::now_v7());
        let loaded = checks::load_snmp_secrets(&pool, &vault(), missing, PVE_01_HOST).await;
        assert!(matches!(loaded, Err(Error::NotFound(_))));
    }
}
