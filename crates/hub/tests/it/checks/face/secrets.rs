//! HTTP check secrets: sealed apart from `config`, bound to their check, opened only for the
//! probe, and kept on update when left empty (SEC-003, SEC-005, LOG-007).

use anima_hub::{
    checks::{
        self, CheckChange, Definition, Error,
        config::{CheckConfig, HttpCredentials, HttpInput, HttpMethod, HttpSecrets, StatusRange},
    },
    credentials::{Sealed, Secret, Vault},
    ids::CheckId,
};
use sqlx::PgPool;

use super::{PVE_01, create, new_check, vault, vault_with};

macro_rules! devices_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const PASSWORD: &str = "pw-SECRET-4711";
const COOKIE: &str = "session=cookie-SECRET-0815";
const TOKEN: &str = "tok-SECRET-9000";

fn http(url: &str, cookie: &str, credentials: HttpCredentials) -> Definition {
    Definition::Http(HttpInput {
        url: url.to_owned(),
        method: HttpMethod::Get,
        headers: vec![
            ("Cookie".to_owned(), Secret::from(cookie.to_owned())),
            ("X-Plain".to_owned(), Secret::from("visible".to_owned())),
        ],
        body: None,
        max_redirects: 3,
        accepted_status: vec![StatusRange { from: 200, to: 299 }],
        keyword: None,
        json_query: None,
        json_expected: None,
        credentials,
        verify_tls: true,
        timeout_ms: 5000,
    })
}

fn basic(password: &str) -> HttpCredentials {
    HttpCredentials::Basic {
        username: "operator".to_owned(),
        password: Secret::from(password.to_owned()),
    }
}

fn text(secret: &Secret) -> &str {
    std::str::from_utf8(secret.expose()).unwrap()
}

/// The check's secrets as its probe opens them, without the configuration they go with.
async fn open_secrets(
    pool: &PgPool,
    vault: &Vault,
    id: CheckId,
) -> Result<Option<HttpSecrets>, Error> {
    let setup = checks::load_http_secrets(pool, vault, id).await?;
    Ok(setup.and_then(|setup| setup.secrets))
}

async fn secrets(pool: &PgPool, id: CheckId) -> HttpSecrets {
    open_secrets(pool, &vault(), id).await.unwrap().unwrap()
}

async fn update(pool: &PgPool, id: CheckId, definition: Definition) -> Result<(), Error> {
    let mut conn = pool.acquire().await.unwrap();
    let change = CheckChange {
        definition: Some(definition),
        ..CheckChange::default()
    };
    checks::update_check(&mut conn, &vault(), id, change)
        .await
        .map(|_| ())
}

async fn web_check(pool: &PgPool, name: &str) -> CheckId {
    let definition = http("https://pve-01.home.lan:8006/", COOKIE, basic(PASSWORD));
    create(pool, new_check(PVE_01, name, definition)).await
}

devices_test! {
    async fn sec_003_http_secrets_are_sealed_and_appear_nowhere_in_the_row(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let row: String = sqlx::query_scalar("SELECT row_to_json(c)::text FROM checks.checks c WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let in_sealed: bool = sqlx::query_scalar(
            "SELECT position(convert_to($2, 'UTF8') IN secrets_sealed) > 0 \
                 OR position(convert_to($3, 'UTF8') IN secrets_sealed) > 0 \
             FROM checks.checks WHERE id = $1",
        )
        .bind(id)
        .bind(PASSWORD)
        .bind(COOKIE)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!in_sealed);
        for secret in [PASSWORD, COOKIE, "SECRET"] {
            assert!(!row.contains(secret), "{secret} in the row");
        }
        assert!(row.contains("visible"), "the plain header stays in config");
        let check = checks::get(&pool, id).await.unwrap();
        assert!(!format!("{check:?}").contains("SECRET"));

        let opened = secrets(&pool, id).await;
        assert_eq!(opened.password.as_ref().map(text), Some(PASSWORD));
        assert_eq!(opened.headers.iter().map(text).collect::<Vec<_>>(), [COOKIE]);
        assert!(opened.bearer_token.is_none());
    }

    async fn sec_003_sealed_secrets_open_only_for_their_own_check_and_key(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let other = web_check(&pool, "Copy target").await;
        sqlx::query(
            "UPDATE checks.checks SET secrets_sealed = (SELECT secrets_sealed FROM checks.checks WHERE id = $1) \
             WHERE id = $2",
        )
        .bind(id)
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
        let copied = open_secrets(&pool, &vault(), other).await;
        assert!(matches!(copied, Err(Error::Credentials(_))), "a copied row opened");
        let wrong_key = open_secrets(&pool, &vault_with(8), id).await;
        assert!(matches!(wrong_key, Err(Error::Credentials(_))), "another key opened");
        assert!(open_secrets(&pool, &vault(), id).await.unwrap().is_some());
    }

    async fn sec_003_a_check_without_secrets_stores_none(pool: PgPool) {
        let definition = http("http://ha.home.lan:8123/", COOKIE, HttpCredentials::None);
        let Definition::Http(mut input) = definition else { unreachable!() };
        input.headers.remove(0);
        let id = create(&pool, new_check(PVE_01, "Open", Definition::Http(input))).await;
        assert!(open_secrets(&pool, &vault(), id).await.unwrap().is_none());
        let icmp = create(&pool, new_check(PVE_01, "Ping", super::icmp())).await;
        assert!(open_secrets(&pool, &vault(), icmp).await.unwrap().is_none());
    }

    async fn sec_003_an_update_with_empty_secrets_keeps_the_stored_ones(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        update(&pool, id, http("https://PVE-01.home.lan:8006/api/", "", basic(""))).await.unwrap();
        let check = checks::get(&pool, id).await.unwrap();
        let CheckConfig::Http(config) = check.config else { panic!("not http") };
        assert_eq!(config.url, "https://PVE-01.home.lan:8006/api/");
        let kept = secrets(&pool, id).await;
        assert_eq!(kept.password.as_ref().map(text), Some(PASSWORD));
        assert_eq!(kept.headers.iter().map(text).collect::<Vec<_>>(), [COOKIE]);
    }

    async fn sec_003_an_update_without_a_definition_keeps_the_secrets(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let mut conn = pool.acquire().await.unwrap();
        let change = CheckChange { name: Some("Renamed".to_owned()), ..CheckChange::default() };
        checks::update_check(&mut conn, &vault(), id, change).await.unwrap();
        assert_eq!(secrets(&pool, id).await.password.as_ref().map(text), Some(PASSWORD));
    }

    async fn sec_003_an_update_with_new_secrets_replaces_them(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let bearer = HttpCredentials::Bearer(Secret::from(TOKEN.to_owned()));
        update(&pool, id, http("https://pve-01.home.lan:8006/", "session=new", bearer)).await.unwrap();
        let replaced = secrets(&pool, id).await;
        assert!(replaced.password.is_none());
        assert_eq!(replaced.bearer_token.as_ref().map(text), Some(TOKEN));
        assert_eq!(replaced.headers.iter().map(text).collect::<Vec<_>>(), ["session=new"]);
    }

    async fn sec_005_an_empty_secret_for_another_kind_or_header_is_refused(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let bearer = HttpCredentials::Bearer(Secret::from(String::new()));
        let error = update(&pool, id, http("https://pve-01.home.lan:8006/", COOKIE, bearer)).await.unwrap_err();
        assert!(matches!(&error, Error::Validation(e) if e[0].field == "config"), "{error:?}");
        let Definition::Http(mut input) = http("https://pve-01.home.lan:8006/", COOKIE, basic(PASSWORD)) else { unreachable!() };
        input.headers.push(("Authorization".to_owned(), Secret::from(String::new())));
        let error = update(&pool, id, Definition::Http(input)).await.unwrap_err();
        assert!(matches!(&error, Error::Validation(e) if e[0].field == "config"), "{error:?}");
        let mut conn = pool.acquire().await.unwrap();
        let fresh = new_check(PVE_01, "Fresh", http("https://x.lan/", "", basic(PASSWORD)));
        let error = checks::create_check(&mut conn, &vault(), fresh).await.unwrap_err();
        assert!(matches!(&error, Error::Validation(e) if e[0].field == "headers"), "{error:?}");
        assert_eq!(secrets(&pool, id).await.password.as_ref().map(text), Some(PASSWORD));
    }

    async fn sec_005_stored_secrets_never_follow_a_changed_destination(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let with_tls = |url: &str, verify: bool, credentials| {
            let Definition::Http(mut input) = http(url, "", credentials) else { unreachable!() };
            input.verify_tls = verify;
            Definition::Http(input)
        };
        let other_user = HttpCredentials::Basic {
            username: "someone".to_owned(),
            password: Secret::from(String::new()),
        };
        let refused = [
            with_tls("http://attacker.example/collect", false, basic("")),
            with_tls("https://attacker.example:8006/", true, basic("")),
            with_tls("http://pve-01.home.lan:8006/", true, basic("")),
            with_tls("https://pve-01.home.lan:8443/", true, basic("")),
            with_tls("https://pve-01.home.lan:8006/", false, basic("")),
            with_tls("https://pve-01.home.lan:8006/", true, other_user),
        ];
        for definition in refused {
            let error = update(&pool, id, definition).await.unwrap_err();
            assert!(matches!(&error, Error::Validation(e) if e[0].field == "config"), "{error:?}");
        }
        let check = checks::get(&pool, id).await.unwrap();
        let CheckConfig::Http(config) = check.config else { panic!("not http") };
        assert_eq!(config.url, "https://pve-01.home.lan:8006/");
        assert!(config.verify_tls);
    }

    async fn sec_005_secrets_do_not_open_after_the_url_is_changed_in_the_database(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        sqlx::query(
            "UPDATE checks.checks SET config = jsonb_set(config, '{url}', '\"http://attacker.example/\"') \
             WHERE id = $1",
        )
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
        let opened = open_secrets(&pool, &vault(), id).await;
        assert!(matches!(opened, Err(Error::SecretsUnbound)), "the secrets opened for a new origin");
        let renamed = web_check(&pool, "Renamed header").await;
        sqlx::query(
            "UPDATE checks.checks SET config = jsonb_set(config, '{headers,0,name}', '\"Authorization\"') \
             WHERE id = $1",
        )
        .bind(renamed)
        .execute(&pool)
        .await
        .unwrap();
        let opened = open_secrets(&pool, &vault(), renamed).await;
        assert!(matches!(opened, Err(Error::SecretsUnbound)), "the secrets opened for a renamed header");
    }

    async fn sec_005_secrets_do_not_open_after_tls_or_auth_is_changed_in_the_database(pool: PgPool) {
        let edits = [
            ("{verifyTls}", "false"),
            ("{auth,username}", "\"attacker\""),
            ("{auth}", "{\"kind\": \"BEARER\"}"),
        ];
        for (path, value) in edits {
            let id = web_check(&pool, path).await;
            sqlx::query(
                "UPDATE checks.checks SET config = jsonb_set(config, $2::text[], $3::jsonb) \
                 WHERE id = $1",
            )
            .bind(id)
            .bind(path)
            .bind(value)
            .execute(&pool)
            .await
            .unwrap();
            let opened = open_secrets(&pool, &vault(), id).await;
            assert!(matches!(opened, Err(Error::SecretsUnbound)), "the secrets opened after {path}");
        }
    }

    async fn sec_005_secrets_given_without_tls_checks_still_open_when_they_are_turned_on(pool: PgPool) {
        let Definition::Http(mut input) = http("https://pve-01.home.lan:8006/", COOKIE, basic(PASSWORD)) else { unreachable!() };
        input.verify_tls = false;
        let id = create(&pool, new_check(PVE_01, "Self-signed", Definition::Http(input))).await;
        assert!(open_secrets(&pool, &vault(), id).await.unwrap().is_some());
        sqlx::query("UPDATE checks.checks SET config = jsonb_set(config, '{verifyTls}', 'true') WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        assert!(open_secrets(&pool, &vault(), id).await.unwrap().is_some());
    }

    async fn sec_003_the_sealed_value_opens_with_exactly_its_check_context(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let bytes: Vec<u8> = sqlx::query_scalar("SELECT secrets_sealed FROM checks.checks WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let sealed = Sealed::from_bytes(&bytes).unwrap();
        let uuid: uuid::Uuid = id.into();
        let context = format!("checks.checks.http:{}", uuid.hyphenated());
        assert!(vault().open(&sealed, context.as_bytes()).is_ok());
        assert!(vault().open(&sealed, format!("checks.check.http:{uuid}").as_bytes()).is_err());
    }

    async fn log_007_errors_never_echo_a_secret(pool: PgPool) {
        let id = web_check(&pool, "Web UI").await;
        let Definition::Http(mut input) = http("https://pve-01.home.lan:8006/", COOKIE, basic(PASSWORD)) else { unreachable!() };
        input.headers.push(("Cookie".to_owned(), Secret::from(format!("{PASSWORD}\r\nX: y"))));
        let error = update(&pool, id, Definition::Http(input)).await.unwrap_err();
        let shown = format!("{error} {error:?}");
        assert!(!shown.contains("SECRET"), "{shown}");
    }
}
