//! `credentials.secrets` through the public SSH and password functions (SEC-003, SEC-005,
//! DEV-011, DEV-015).

mod batch;
mod concurrency;
mod host;
mod kinds;
mod round_trip;
mod schema;
mod tampering;
mod transactions;

use std::io::Write as _;

use anima_hub::{
    credentials::{Error, PasswordKind, Secret, SshSecret, Vault, store_password, store_ssh},
    ids::DeviceId,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sqlx::PgPool;

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// The device host every secret here is entered for and opened for (SEC-011).
const HOST: &str = "pve-01.lan";
const PASSWORD: &str = "correct horse battery staple";
const KEY: &str =
    "-----BEGIN OPENSSH PRIVATE KEY-----\nthrowaway\n-----END OPENSSH PRIVATE KEY-----\n";
const PASSPHRASE: &str = "open sesame";
const RDP_PASSWORD: &str = "rdp tr0ub4dor";
const VNC_PASSWORD: &str = "vnc 8chars!";

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
    store_ssh(&mut pool.acquire().await.unwrap(), vault, (id, HOST), s).await
}

/// Stores a password sealed with `vault(1)`, the vault these tests open with.
async fn store_pw(
    pool: &PgPool,
    id: DeviceId,
    kind: PasswordKind,
    text: &str,
) -> Result<(), Error> {
    let mut conn = pool.acquire().await.unwrap();
    store_password(&mut conn, &vault(1), (id, HOST), (kind, secret(text))).await
}

async fn kinds(pool: &PgPool, id: DeviceId) -> Vec<String> {
    sqlx::query_scalar("SELECT kind FROM credentials.secrets WHERE device_id = $1 ORDER BY kind")
        .bind(id)
        .fetch_all(pool)
        .await
        .unwrap()
}

fn constraint_of(error: &sqlx::Error) -> Option<&str> {
    error.as_database_error().and_then(|e| e.constraint())
}
