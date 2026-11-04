//! The only SQL for `credentials.secrets` (K2, CQ-032). Rows hold sealed values only (SEC-003).

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::ids::DeviceId;

use super::{Error, Sealed, ssh::SshKind};

const DEVICE_FKEY: &str = "secrets_device_id_fkey";

/// A stored row as read back: the kind text and the sealed columns, not yet checked.
pub(super) struct StoredSecret {
    pub(super) kind: String,
    pub(super) nonce: Vec<u8>,
    pub(super) ciphertext: Vec<u8>,
    pub(super) key_id: String,
}

/// Deletes every SSH row of the device; returns how many rows were removed.
pub(super) async fn delete_ssh(conn: &mut PgConnection, device: DeviceId) -> Result<u64, Error> {
    let done =
        sqlx::query("DELETE FROM credentials.secrets WHERE device_id = $1 AND kind = ANY($2)")
            .bind(device)
            .bind(&SshKind::ALL_SQL[..])
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

/// Reads every SSH row of the device.
pub(super) async fn fetch_ssh(db: &PgPool, device: DeviceId) -> Result<Vec<StoredSecret>, Error> {
    let rows: Vec<(String, Vec<u8>, Vec<u8>, String)> = sqlx::query_as(
        "SELECT kind, nonce, ciphertext, key_id FROM credentials.secrets \
         WHERE device_id = $1 AND kind = ANY($2)",
    )
    .bind(device)
    .bind(&SshKind::ALL_SQL[..])
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(kind, nonce, ciphertext, key_id)| StoredSecret {
            kind,
            nonce,
            ciphertext,
            key_id,
        })
        .collect())
}

/// Whether the device has any SSH row.
pub(super) async fn exists_ssh(db: &PgPool, device: DeviceId) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM credentials.secrets WHERE device_id = $1 AND kind = ANY($2))",
    )
    .bind(device)
    .bind(&SshKind::ALL_SQL[..])
    .fetch_one(db)
    .await?)
}
