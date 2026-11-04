//! The only SQL for `credentials.secrets`, the one table this domain owns (CQ-032). Rows hold sealed values only (SEC-003).
//!
//! The table's kind check admits only SSH kinds, so "the device's rows" are its SSH rows.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::ids::DeviceId;

use super::{Error, Sealed, ssh::SshKind};

const DEVICE_FKEY: &str = "secrets_device_id_fkey";

/// A stored row as read back: the kind text and the sealed columns, not yet checked.
#[derive(sqlx::FromRow)]
pub(super) struct StoredSecret {
    pub(super) kind: String,
    pub(super) nonce: Vec<u8>,
    pub(super) ciphertext: Vec<u8>,
    pub(super) key_id: String,
}

/// Waits for, then holds until the transaction ends, this domain's advisory lock on the
/// device's secrets, so concurrent writers for one device run one after the other.
pub(super) async fn lock_device(conn: &mut PgConnection, device: DeviceId) -> Result<(), Error> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('credentials.secrets:' || $1::text, 0))",
    )
    .bind(device)
    .execute(conn)
    .await?;
    Ok(())
}

/// Deletes every row of the device; returns how many rows were removed.
pub(super) async fn delete_device(conn: &mut PgConnection, device: DeviceId) -> Result<u64, Error> {
    let done = sqlx::query("DELETE FROM credentials.secrets WHERE device_id = $1")
        .bind(device)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}

/// Inserts one sealed row with a fresh UUID v7 id.
///
/// # Errors
///
/// [`Error::UnknownDevice`] when no device has this id.
pub(super) async fn insert(
    conn: &mut PgConnection,
    device: DeviceId,
    kind: SshKind,
    sealed: &Sealed,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO credentials.secrets (id, device_id, kind, nonce, ciphertext, key_id) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(device)
    .bind(kind.as_sql())
    .bind(sealed.nonce())
    .bind(sealed.ciphertext())
    .bind(sealed.key_id())
    .execute(conn)
    .await
    .map_err(|error| {
        let constraint = error.as_database_error().and_then(|e| e.constraint());
        if constraint == Some(DEVICE_FKEY) {
            Error::UnknownDevice
        } else {
            Error::Database(error)
        }
    })?;
    Ok(())
}

/// Reads every row of the device.
pub(super) async fn fetch_device(
    db: &PgPool,
    device: DeviceId,
) -> Result<Vec<StoredSecret>, Error> {
    Ok(sqlx::query_as(
        "SELECT kind, nonce, ciphertext, key_id FROM credentials.secrets WHERE device_id = $1",
    )
    .bind(device)
    .fetch_all(db)
    .await?)
}

/// Whether the device has any row.
pub(super) async fn exists_for_device(db: &PgPool, device: DeviceId) -> Result<bool, Error> {
    Ok(
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM credentials.secrets WHERE device_id = $1)",
        )
        .bind(device)
        .fetch_one(db)
        .await?,
    )
}
