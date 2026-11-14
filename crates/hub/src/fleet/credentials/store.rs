//! The only SQL for `credentials.secrets` and `credentials.source_secrets`, the tables this
//! domain owns (CQ-032). Rows hold sealed values only (SEC-003).
//!
//! The table holds SSH parts and RDP and VNC passwords side by side. Every read, check and
//! delete here takes the kinds it may match, so a caller sees and removes only its own kinds.
//! The advisory lock is per device, not per kind, so writers of every kind share one lock order.

use std::collections::HashSet;

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::ids::{DeviceId, SourceAccountId, SourceSecretId};

use super::{Error, Sealed};

const DEVICE_FKEY: &str = "secrets_device_id_fkey";

/// A stored row as read back: the kind text and the sealed columns, not yet checked.
#[derive(sqlx::FromRow)]
pub(super) struct StoredSecret {
    pub(super) kind: String,
    pub(super) nonce: Vec<u8>,
    pub(super) ciphertext: Vec<u8>,
    pub(super) key_id: String,
    /// The host migration 0220 recorded for a row sealed before its host was sealed with it
    /// (SEC-011); `None` for every row sealed since.
    pub(super) legacy_host: Option<String>,
}

/// Waits for, then holds until the transaction ends, this domain's advisory lock on the
/// device's secrets of every kind, so concurrent writers for one device run one after the other.
pub(super) async fn lock_device(conn: &mut PgConnection, device: DeviceId) -> Result<(), Error> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('credentials.secrets:' || $1::text, 0))",
    )
    .bind(device)
    .execute(conn)
    .await?;
    Ok(())
}

/// Deletes the device's rows of the given kinds; returns how many rows were removed.
pub(super) async fn delete_kinds(
    conn: &mut PgConnection,
    device: DeviceId,
    kinds: &[&str],
) -> Result<u64, Error> {
    let done =
        sqlx::query("DELETE FROM credentials.secrets WHERE device_id = $1 AND kind = ANY($2)")
            .bind(device)
            .bind(kinds)
            .execute(conn)
            .await?;
    Ok(done.rows_affected())
}

/// Inserts one sealed row of `kind` with a fresh UUID v7 id and no `legacy_host`: the value
/// holds its host (SEC-011).
///
/// # Errors
///
/// [`Error::UnknownDevice`] when no device has this id.
pub(super) async fn insert(
    conn: &mut PgConnection,
    device: DeviceId,
    kind: &str,
    sealed: &Sealed,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO credentials.secrets (id, device_id, kind, nonce, ciphertext, key_id) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(device)
    .bind(kind)
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

/// Reads the device's rows of the given kinds.
pub(super) async fn fetch_kinds(
    db: &PgPool,
    device: DeviceId,
    kinds: &[&str],
) -> Result<Vec<StoredSecret>, Error> {
    Ok(sqlx::query_as(
        "SELECT kind, nonce, ciphertext, key_id, legacy_host FROM credentials.secrets \
         WHERE device_id = $1 AND kind = ANY($2)",
    )
    .bind(device)
    .bind(kinds)
    .fetch_all(db)
    .await?)
}

/// Whether the device has a row of any of the given kinds.
pub(super) async fn exists_of_kinds(
    db: &PgPool,
    device: DeviceId,
    kinds: &[&str],
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM credentials.secrets \
         WHERE device_id = $1 AND kind = ANY($2))",
    )
    .bind(device)
    .bind(kinds)
    .fetch_one(db)
    .await?)
}

/// Which of the devices have a row of any of the given kinds, in one query. An empty slice
/// matches nothing.
pub(super) async fn devices_with_kinds(
    db: &PgPool,
    devices: &[DeviceId],
    kinds: &[&str],
) -> Result<HashSet<DeviceId>, Error> {
    let found: Vec<DeviceId> = sqlx::query_scalar(
        "SELECT DISTINCT device_id FROM credentials.secrets \
         WHERE device_id = ANY($1) AND kind = ANY($2)",
    )
    .bind(devices)
    .bind(kinds)
    .fetch_all(db)
    .await?;
    Ok(found.into_iter().collect())
}

/// A stored source secret as read back: the sealed columns, not yet checked.
#[derive(sqlx::FromRow)]
pub(super) struct StoredSourceSecret {
    pub(super) nonce: Vec<u8>,
    pub(super) ciphertext: Vec<u8>,
    pub(super) key_id: String,
}

/// Inserts the account's sealed source secret, or replaces the sealed columns of the one it
/// has, keeping that row's id; returns the row's id. One statement, so concurrent stores for
/// one account leave exactly one row.
pub(super) async fn upsert_source(
    conn: &mut PgConnection,
    account: SourceAccountId,
    sealed: &Sealed,
) -> Result<SourceSecretId, Error> {
    Ok(sqlx::query_scalar(
        "INSERT INTO credentials.source_secrets (id, account_id, nonce, ciphertext, key_id) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (account_id) DO UPDATE SET nonce = EXCLUDED.nonce, \
         ciphertext = EXCLUDED.ciphertext, key_id = EXCLUDED.key_id \
         RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(account)
    .bind(sealed.nonce())
    .bind(sealed.ciphertext())
    .bind(sealed.key_id())
    .fetch_one(conn)
    .await?)
}

/// Reads the sealed columns of the source secret with this id.
pub(super) async fn fetch_source(
    db: &PgPool,
    id: SourceSecretId,
) -> Result<Option<StoredSourceSecret>, Error> {
    Ok(sqlx::query_as(
        "SELECT nonce, ciphertext, key_id FROM credentials.source_secrets WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?)
}

/// Deletes the source secret with this id; returns whether a row was removed.
pub(super) async fn delete_source(
    conn: &mut PgConnection,
    id: SourceSecretId,
) -> Result<bool, Error> {
    let done = sqlx::query("DELETE FROM credentials.source_secrets WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}
