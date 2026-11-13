//! SNMP check secrets are bound to the device host they were entered for (SEC-010, security
//! review 2026-11-13): they open only for a probe of that host text, an update after a host
//! change asks for them again, and migration 0324 binds secrets sealed before it to the host
//! their device had then.

use anima_hub::{
    fleet::{
        checks::{
            self, CheckChange, Definition, Error, Sealer,
            config::{SnmpConfig, SnmpCredentials, SnmpInput, SnmpSecrets, SnmpVersion},
        },
        credentials::Secret,
    },
    ids::CheckId,
};
use sqlx::PgPool;

use super::{PVE_01, PVE_01_HOST, create, device, new_check, vault};

macro_rules! devices_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const COMMUNITY: &str = "community-SECRET-host-1";
/// An address inside the permitted networks that someone who may update devices chose.
const CHOSEN: &str = "203.0.113.9";

fn definition(community: Option<&str>) -> Definition {
    Definition::Snmp(SnmpInput {
        config: SnmpConfig {
            port: 161,
            version: SnmpVersion::V2c,
            oid: "1.3.6.1.2.1.1.5.0".to_owned(),
            expected: None,
            username: None,
            timeout_ms: 5000,
        },
        credentials: SnmpCredentials {
            community: community.map(|text| Secret::from(text.to_owned())),
            ..SnmpCredentials::default()
        },
    })
}

async fn snmp_check(pool: &PgPool) -> CheckId {
    create(pool, new_check(PVE_01, "SNMP", definition(Some(COMMUNITY)))).await
}

async fn move_device(pool: &PgPool, host: &str) {
    sqlx::query("UPDATE devices.devices SET host = $2 WHERE id = $1")
        .bind(device(PVE_01))
        .bind(host)
        .execute(pool)
        .await
        .unwrap();
}

/// The community that opens for a probe of `host`.
async fn community_for(pool: &PgPool, id: CheckId, host: &str) -> Result<String, Error> {
    let setup = checks::load_snmp_secrets(pool, &vault(), id, host).await?;
    match setup.and_then(|setup| setup.secrets) {
        Some(SnmpSecrets::Community(community)) => {
            Ok(String::from_utf8(community.expose().to_vec()).unwrap())
        }
        _ => panic!("no community"),
    }
}

/// Changes the definition as an operator would, for the device's host `host`.
async fn update(
    pool: &PgPool,
    id: CheckId,
    host: &str,
    community: Option<&str>,
) -> Result<(), Error> {
    let mut conn = pool.acquire().await.unwrap();
    let change = CheckChange {
        definition: Some(definition(community)),
        ..CheckChange::default()
    };
    let sealer = Sealer {
        vault: &vault(),
        host,
    };
    checks::update_check(&mut conn, sealer, id, change)
        .await
        .map(|_| ())
}

/// The secrets sealed as the hub sealed them before SEC-010: a binding with no host.
fn sealed_without_host(id: CheckId) -> Vec<u8> {
    let binding = br#"{"version":"V2C","username":null}"#;
    let mut plain = u32::try_from(binding.len()).unwrap().to_be_bytes().to_vec();
    plain.extend_from_slice(binding);
    plain.push(1); // a community
    plain.extend_from_slice(&u16::try_from(COMMUNITY.len()).unwrap().to_be_bytes());
    plain.extend_from_slice(COMMUNITY.as_bytes());
    let context = format!("checks.checks.snmp:{id}");
    vault()
        .seal(&Secret::new(plain), context.as_bytes())
        .unwrap()
        .to_bytes()
}

/// The statement of migration 0324 that binds existing rows.
fn binding_statement() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/0324_checks_snmp_secret_host.sql"
    );
    let sql = std::fs::read_to_string(path).unwrap();
    let start = sql.find("update checks.checks").unwrap();
    let end = start + sql[start..].find(';').unwrap();
    sql[start..end].to_owned()
}

async fn legacy_host(pool: &PgPool, id: CheckId) -> Option<String> {
    sqlx::query_scalar("SELECT snmp_legacy_host FROM checks.checks WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

devices_test! {
    async fn sec_010_snmp_secrets_open_only_for_the_host_they_were_entered_for(pool: PgPool) {
        let id = snmp_check(&pool).await;
        assert_eq!(community_for(&pool, id, PVE_01_HOST).await.unwrap(), COMMUNITY);
        // Host names compare without letter case; the binding never looks at an address, so a
        // name whose address changes keeps its secrets.
        assert_eq!(community_for(&pool, id, "PVE-01.Home.LAN").await.unwrap(), COMMUNITY);
        move_device(&pool, CHOSEN).await;
        let moved = checks::load_snmp_secrets(&pool, &vault(), id, CHOSEN).await;
        assert!(matches!(moved, Err(Error::SecretsForOtherHost)));
        let moved = checks::load_snmp_secrets(&pool, &vault(), id, "pve-01.home.lan.evil").await;
        assert!(matches!(moved, Err(Error::SecretsForOtherHost)));
        assert_eq!(legacy_host(&pool, id).await, None, "a new secret holds its host itself");
    }

    async fn sec_010_an_update_after_a_host_change_asks_for_the_secrets_again(pool: PgPool) {
        let id = snmp_check(&pool).await;
        move_device(&pool, CHOSEN).await;
        let kept = update(&pool, id, CHOSEN, None).await;
        let Err(Error::Validation(errors)) = kept else { panic!("the secrets were kept") };
        assert_eq!(errors.iter().map(|e| e.field).collect::<Vec<_>>(), ["community"]);
        assert!(!errors[0].message.contains("SECRET"));
        // Nothing changed: the secrets still open only for the host they were entered for.
        assert_eq!(community_for(&pool, id, PVE_01_HOST).await.unwrap(), COMMUNITY);
        update(&pool, id, CHOSEN, Some("entered-again-SECRET")).await.unwrap();
        assert_eq!(community_for(&pool, id, CHOSEN).await.unwrap(), "entered-again-SECRET");
        let old = checks::load_snmp_secrets(&pool, &vault(), id, PVE_01_HOST).await;
        assert!(matches!(old, Err(Error::SecretsForOtherHost)));
    }

    async fn sec_010_the_migration_binds_older_secrets_to_the_host_of_their_device(pool: PgPool) {
        let id = snmp_check(&pool).await;
        let ping = create(&pool, new_check(PVE_01, "Ping", super::icmp())).await;
        sqlx::query("UPDATE checks.checks SET secrets_sealed = $2 WHERE id = $1")
            .bind(id)
            .bind(sealed_without_host(id))
            .execute(&pool)
            .await
            .unwrap();
        // With no host in the binding and none recorded, the secrets are sent nowhere.
        let unbound = checks::load_snmp_secrets(&pool, &vault(), id, PVE_01_HOST).await;
        assert!(matches!(unbound, Err(Error::SecretsUnbound)));

        sqlx::query(&binding_statement()).execute(&pool).await.unwrap();

        assert_eq!(legacy_host(&pool, id).await.as_deref(), Some(PVE_01_HOST));
        assert_eq!(legacy_host(&pool, ping).await, None, "not an SNMP check");
        assert_eq!(community_for(&pool, id, PVE_01_HOST).await.unwrap(), COMMUNITY);
        move_device(&pool, CHOSEN).await;
        let moved = checks::load_snmp_secrets(&pool, &vault(), id, CHOSEN).await;
        assert!(matches!(moved, Err(Error::SecretsForOtherHost)));
        // Entering the community again seals it with its host and clears the recorded one.
        update(&pool, id, CHOSEN, Some(COMMUNITY)).await.unwrap();
        assert_eq!(legacy_host(&pool, id).await, None);
        assert_eq!(community_for(&pool, id, CHOSEN).await.unwrap(), COMMUNITY);
    }
}
