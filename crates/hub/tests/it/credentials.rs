//! `credentials.secrets` through the public SSH functions (SEC-003, R-010, DEV-015).

use std::io::Write as _;

use anima_hub::{
    credentials::{Error, Secret, SshSecret, Vault, clear_ssh, has_ssh, load_ssh, store_ssh},
    ids::DeviceId,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sqlx::{Connection as _, PgPool};

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const PASSWORD: &str = "correct horse battery staple";
const KEY: &str =
    "-----BEGIN OPENSSH PRIVATE KEY-----\nthrowaway\n-----END OPENSSH PRIVATE KEY-----\n";
const PASSPHRASE: &str = "open sesame";

/// A vault over a throwaway key written to a temporary file (SEC-004), never a real key.
fn vault(fill: u8) -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([fill; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

fn secret(text: &str) -> Secret {
    Secret::from(text.to_owned())
}

fn password() -> SshSecret {
    SshSecret::Password(secret(PASSWORD))
}

fn private_key(passphrase: Option<&str>) -> SshSecret {
    SshSecret::PrivateKey {
        key: secret(KEY),
        passphrase: passphrase.map(secret),
    }
}

async fn store(pool: &PgPool, vault: &Vault, id: DeviceId, s: SshSecret) -> Result<(), Error> {
    store_ssh(&mut pool.acquire().await.unwrap(), vault, id, s).await
}

async fn kinds(pool: &PgPool, id: DeviceId) -> Vec<String> {
    sqlx::query_scalar("SELECT kind FROM credentials.secrets WHERE device_id = $1 ORDER BY kind")
        .bind(id)
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn sec_003_stored_row_holds_no_plaintext(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, password()).await.unwrap();

    let (nonce, ciphertext, key_id): (Vec<u8>, Vec<u8>, String) = sqlx::query_as(
        "SELECT nonce, ciphertext, key_id FROM credentials.secrets WHERE device_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let plaintext = PASSWORD.as_bytes();
    assert_eq!(nonce.len(), 24);
    assert_ne!(ciphertext, plaintext);
    assert!(!ciphertext.windows(plaintext.len()).any(|w| w == plaintext));
    assert_eq!(key_id, vault.seal(&secret(""), b"").key_id());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn sec_003_password_round_trips(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, password()).await.unwrap();
    let loaded = load_ssh(&pool, &vault, id).await.unwrap();
    assert!(matches!(loaded, Some(SshSecret::Password(p)) if p.expose() == PASSWORD.as_bytes()));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn sec_003_private_key_round_trips(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(None)).await.unwrap();
    let loaded = load_ssh(&pool, &vault, id).await.unwrap();
    assert!(matches!(
        loaded,
        Some(SshSecret::PrivateKey { key, passphrase: None }) if key.expose() == KEY.as_bytes()
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn sec_003_private_key_with_passphrase_round_trips(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    let loaded = load_ssh(&pool, &vault, id).await.unwrap();
    assert!(matches!(
        loaded,
        Some(SshSecret::PrivateKey { key, passphrase: Some(p) })
            if key.expose() == KEY.as_bytes() && p.expose() == PASSPHRASE.as_bytes()
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn sec_003_password_replaces_key_and_passphrase(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    assert_eq!(
        kinds(&pool, id).await,
        ["ssh_passphrase", "ssh_private_key"]
    );

    store(&pool, &vault, id, password()).await.unwrap();
    assert_eq!(kinds(&pool, id).await, ["ssh_password"]);
    let loaded = load_ssh(&pool, &vault, id).await.unwrap();
    assert!(matches!(loaded, Some(SshSecret::Password(p)) if p.expose() == PASSWORD.as_bytes()));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn has_ssh_follows_store_and_clear(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    assert!(!has_ssh(&pool, id).await.unwrap());
    assert!(load_ssh(&pool, &vault, id).await.unwrap().is_none());

    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    assert!(has_ssh(&pool, id).await.unwrap());
    assert!(!has_ssh(&pool, device(DOCKER_HOST)).await.unwrap());

    let mut conn = pool.acquire().await.unwrap();
    assert!(clear_ssh(&mut conn, id).await.unwrap());
    assert!(!clear_ssh(&mut conn, id).await.unwrap());
    assert!(!has_ssh(&pool, id).await.unwrap());
    assert!(load_ssh(&pool, &vault, id).await.unwrap().is_none());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn sec_003_row_moved_to_another_device_fails_to_open(pool: PgPool) {
    let (vault, from, to) = (vault(1), device(PVE_01), device(DOCKER_HOST));
    store(&pool, &vault, from, password()).await.unwrap();
    sqlx::query("UPDATE credentials.secrets SET device_id = $2 WHERE device_id = $1")
        .bind(from)
        .bind(to)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        load_ssh(&pool, &vault, to).await,
        Err(Error::Decrypt)
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn passphrase_without_its_key_is_malformed(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM credentials.secrets WHERE device_id = $1 AND kind = 'ssh_private_key'",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        load_ssh(&pool, &vault, id).await,
        Err(Error::Malformed)
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn sec_003_other_vault_key_fails(pool: PgPool) {
    let id = device(PVE_01);
    store(&pool, &vault(1), id, password()).await.unwrap();
    assert!(matches!(
        load_ssh(&pool, &vault(2), id).await,
        Err(Error::WrongKey)
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn store_for_unknown_device_fails(pool: PgPool) {
    let unknown = DeviceId::from(uuid::Uuid::now_v7());
    let result = store(&pool, &vault(1), unknown, password()).await;
    assert!(matches!(result, Err(Error::UnknownDevice)));
    assert_eq!(kinds(&pool, unknown).await, Vec::<String>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn store_failure_keeps_the_callers_transaction_usable(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let mut tx = pool.begin().await.unwrap();
    let unknown = DeviceId::from(uuid::Uuid::now_v7());
    let failed = store_ssh(&mut tx, &vault, unknown, password()).await;
    assert!(matches!(failed, Err(Error::UnknownDevice)));
    store_ssh(&mut tx, &vault, id, password()).await.unwrap();
    tx.commit().await.unwrap();
    assert!(has_ssh(&pool, id).await.unwrap());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn dev_015_deleting_the_device_deletes_its_secrets(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    // devices exposes no delete function yet; this deletes the row the way it will.
    sqlx::query("DELETE FROM devices.devices WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(kinds(&pool, id).await, Vec::<String>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn store_in_a_rolled_back_transaction_leaves_nothing(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let mut conn = pool.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    store_ssh(&mut tx, &vault, id, password()).await.unwrap();
    tx.rollback().await.unwrap();
    assert!(!has_ssh(&pool, id).await.unwrap());
}
