//! The world the link and probe tests start from, which `sessions` tests start from too: the
//! operator, `docker-host` with an accepted host key and a stored password, and a vault.

use std::io::Write as _;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    credentials::{self, Secret, SshSecret, Vault},
    ids::DeviceId,
    trust::PublicKey,
};

pub(crate) const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";

pub(crate) const OPERATOR: &str = "0190a000-0000-7000-8000-000000000001";

pub(crate) const PASSWORD: &str = "correct horse battery staple";

/// The address the test client connects from, and the device's SSH server.
pub(crate) const LOOPBACK: &str = "127.0.0.1";

pub(crate) fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

pub(crate) fn docker_host() -> DeviceId {
    DOCKER_HOST.parse().unwrap()
}

/// A vault over a throwaway key in a temporary file (SEC-004).
pub(crate) fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// The operator every session belongs to.
pub(crate) async fn add_operator(pool: &PgPool) {
    sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::text::uuid, 'operator', '$argon2id$x', now())",
    )
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
}

/// The operator, `accepted` as `docker-host`'s host key, and [`PASSWORD`] for it.
pub(crate) async fn prepare(pool: &PgPool, vault: &Vault, accepted: &PublicKey) {
    add_operator(pool).await;
    sqlx::query(
        "insert into trust.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1::text::uuid, $2, $3, $4, $5, $5, $6::text::uuid)",
    )
    .bind(DOCKER_HOST)
    .bind(accepted.algorithm())
    .bind(accepted.fingerprint().as_str().to_owned())
    .bind(accepted.to_base64())
    .bind(now())
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
    store_password(pool, vault, PASSWORD).await;
}

/// Stores `password` as `docker-host`'s SSH credential, replacing the one stored. It is bound
/// to [`LOOPBACK`], where the tests' device servers listen and where they move the device
/// before it is reached (SEC-011).
pub(crate) async fn store_password(pool: &PgPool, vault: &Vault, password: &str) {
    let mut conn = pool.acquire().await.unwrap();
    let password = SshSecret::Password(Secret::from(password.to_owned()));
    credentials::store_ssh(&mut conn, vault, (docker_host(), LOOPBACK), password)
        .await
        .unwrap();
}

/// Points `docker-host` at `host` and `port`, as an operator's edit would.
pub(crate) async fn move_device(pool: &PgPool, host: &str, port: u16) {
    sqlx::query("update devices.devices set host = $2 where id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(host)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("update devices.ssh_settings set port = $2 where device_id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(i32::from(port))
        .execute(pool)
        .await
        .unwrap();
}
