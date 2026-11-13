//! Credentials bound to the device host they were entered for (SEC-011), through the public
//! functions and migration 0220.

use anima_hub::fleet::credentials::{
    Error, PasswordKind, Secret, SshSecret, load_password, load_ssh, store_password,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    HOST, PASSWORD, PVE_01, RDP_PASSWORD, device, password, secret, store, store_pw, vault,
};

/// PVE-01's host in the fixture.
const PVE_01_HOST: &str = "pve-01.home.lan";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_011_a_secret_opens_only_for_its_host_and_stays_stored(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, password()).await.unwrap();
    store_pw(&pool, id, PasswordKind::Vnc, RDP_PASSWORD)
        .await
        .unwrap();
    for other in ["collector.example.net", "pve-01.lan.", "203.0.113.9"] {
        assert!(matches!(
            load_ssh(&pool, &vault, (id, other)).await,
            Err(Error::EnteredForOtherHost)
        ));
        assert!(matches!(
            load_password(&pool, &vault, (id, other), PasswordKind::Vnc).await,
            Err(Error::EnteredForOtherHost)
        ));
    }
    // Kept, not removed: the host it was entered for, in any letter case, still opens it.
    let loaded = load_ssh(&pool, &vault, (id, &HOST.to_uppercase()))
        .await
        .unwrap();
    assert!(matches!(loaded, Some(SshSecret::Password(p)) if p.expose() == PASSWORD.as_bytes()));
    let vnc = load_password(&pool, &vault, (id, HOST), PasswordKind::Vnc)
        .await
        .unwrap();
    assert_eq!(vnc.unwrap().expose(), RDP_PASSWORD.as_bytes());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_011_storing_again_binds_the_secret_to_the_new_host(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store_pw(&pool, id, PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let again = (PasswordKind::Rdp, secret("entered again"));
    store_password(&mut conn, &vault, (id, "new.home.lan"), again)
        .await
        .unwrap();
    let opened = load_password(&pool, &vault, (id, "new.home.lan"), PasswordKind::Rdp)
        .await
        .unwrap();
    assert_eq!(opened.unwrap().expose(), b"entered again");
    assert!(matches!(
        load_password(&pool, &vault, (id, HOST), PasswordKind::Rdp).await,
        Err(Error::EnteredForOtherHost)
    ));
}

/// The statement of migration 0220 that binds existing rows.
fn binding_statement() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/0220_credentials_secret_host.sql"
    );
    let sql = std::fs::read_to_string(path).unwrap();
    let start = sql.find("update credentials.secrets").unwrap();
    let end = start + sql[start..].find(';').unwrap();
    sql[start..end].to_owned()
}

/// Inserts a VNC password sealed as the hub sealed it before SEC-011: the secret alone, under
/// the context without a host.
async fn insert_from_before(pool: &PgPool, text: &str) {
    let id = device(PVE_01);
    let context = format!("credentials.secrets:{id}:vnc_password");
    let sealed = vault(1)
        .seal(&Secret::from(text.to_owned()), context.as_bytes())
        .unwrap();
    sqlx::query(
        "insert into credentials.secrets (id, device_id, kind, nonce, ciphertext, key_id) \
         values ($1, $2, 'vnc_password', $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(id)
    .bind(sealed.nonce())
    .bind(sealed.ciphertext())
    .bind(sealed.key_id())
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_011_the_migration_binds_older_rows_to_the_host_of_their_device(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    insert_from_before(&pool, "from before").await;
    // With no host sealed in and none recorded, the row opens for no host.
    assert!(matches!(
        load_password(&pool, &vault, (id, PVE_01_HOST), PasswordKind::Vnc).await,
        Err(Error::Decrypt)
    ));

    sqlx::query(&binding_statement())
        .execute(&pool)
        .await
        .unwrap();

    let opened = load_password(&pool, &vault, (id, PVE_01_HOST), PasswordKind::Vnc)
        .await
        .unwrap();
    assert_eq!(opened.unwrap().expose(), b"from before");
    assert!(matches!(
        load_password(
            &pool,
            &vault,
            (id, "collector.example.net"),
            PasswordKind::Vnc
        )
        .await,
        Err(Error::EnteredForOtherHost)
    ));
    // Storing it again holds the host in the value and clears the recorded one.
    store_pw(&pool, id, PasswordKind::Vnc, "entered again")
        .await
        .unwrap();
    let recorded: Option<String> =
        sqlx::query_scalar("select legacy_host from credentials.secrets where device_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(recorded, None);
}

/// A database writer who records another host on a row that holds its own does not rebind it.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_011_a_recorded_host_does_not_rebind_a_new_row(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store_pw(&pool, id, PasswordKind::Vnc, RDP_PASSWORD)
        .await
        .unwrap();
    sqlx::query("update credentials.secrets set legacy_host = 'collector.example.net'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        load_password(
            &pool,
            &vault,
            (id, "collector.example.net"),
            PasswordKind::Vnc
        )
        .await,
        Err(Error::Decrypt)
    ));
}
