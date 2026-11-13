//! All SQL on schema `trust`: I/O only. Ids and time come in as parameters.

mod server_certificates;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::{
    Error,
    model::HostKey,
    public_key::{BadKey, PublicKey},
};
use crate::ids::{DeviceId, OperatorId};

pub(super) use server_certificates::{
    AcceptedCertificate, lock_server_certificate, put_server_certificate, server_certificates,
};

/// The stored algorithm differs from the one inside the stored key.
#[derive(Debug, thiserror::Error)]
#[error("host key row names another algorithm than its key")]
struct AlgorithmMismatch;

fn decode<E: std::error::Error + Send + Sync + 'static>(error: E) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(error))
}

#[derive(sqlx::FromRow)]
struct HostKeyRow {
    device_id: DeviceId,
    algorithm: String,
    public_key: String,
    first_seen_at: DateTime<Utc>,
}

impl TryFrom<HostKeyRow> for (DeviceId, HostKey) {
    type Error = sqlx::Error;

    fn try_from(row: HostKeyRow) -> Result<Self, sqlx::Error> {
        let public_key =
            PublicKey::from_base64(&row.public_key).map_err(|error: BadKey| decode(error))?;
        if public_key.algorithm() != row.algorithm {
            return Err(decode(AlgorithmMismatch));
        }
        let key = HostKey {
            public_key,
            first_seen_at: row.first_seen_at,
        };
        Ok((row.device_id, key))
    }
}

/// The accepted host key of each listed device that has one, in one query (API-005).
pub(super) async fn host_keys<'e>(
    db: impl PgExecutor<'e>,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, HostKey>, Error> {
    if devices.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<HostKeyRow> = sqlx::query_as(
        "select device_id, algorithm, public_key, first_seen_at from trust.host_keys \
         where device_id = any($1)",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    let keys = rows
        .into_iter()
        .map(<(DeviceId, HostKey)>::try_from)
        .collect::<Result<_, _>>()?;
    Ok(keys)
}

/// Waits for, then holds until the transaction ends, this domain's advisory lock on the
/// device's host key, so two accepts for one device run one after the other and each audit
/// entry names the key it replaced.
pub(super) async fn lock_host_key(conn: &mut PgConnection, device: DeviceId) -> Result<(), Error> {
    sqlx::query(
        "select pg_advisory_xact_lock(hashtextextended('trust.host_keys:' || $1::text, 0))",
    )
    .bind(device)
    .execute(conn)
    .await?;
    Ok(())
}

/// A host key the operator accepts for a device.
pub(super) struct AcceptedKey<'a> {
    pub(super) device: DeviceId,
    pub(super) key: &'a HostKey,
    pub(super) by: OperatorId,
    pub(super) at: DateTime<Utc>,
}

/// Stores the accepted key, replacing the device's previous one. The algorithm and the
/// fingerprint are read from the key, so the row's checks always hold.
pub(super) async fn put_host_key(
    conn: &mut PgConnection,
    accepted: &AcceptedKey<'_>,
) -> Result<(), Error> {
    let public_key = &accepted.key.public_key;
    let stored = sqlx::query(
        "insert into trust.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1, $2, $3, $4, $5, $6, $7) \
         on conflict (device_id) do update set algorithm = excluded.algorithm, \
             fingerprint_sha256 = excluded.fingerprint_sha256, public_key = excluded.public_key, \
             first_seen_at = excluded.first_seen_at, accepted_at = excluded.accepted_at, \
             accepted_by = excluded.accepted_by",
    )
    .bind(accepted.device)
    .bind(public_key.algorithm())
    .bind(public_key.fingerprint().as_str())
    .bind(public_key.to_base64())
    .bind(accepted.key.first_seen_at)
    .bind(accepted.at)
    .bind(accepted.by)
    .execute(conn)
    .await;
    match stored {
        Ok(_) => Ok(()),
        Err(error) if is_violation_of(&error, "host_keys_device_id_fkey") => {
            Err(Error::NotFound(accepted.device))
        }
        Err(error) => Err(error.into()),
    }
}

fn is_violation_of(error: &sqlx::Error, constraint: &str) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.constraint())
        == Some(constraint)
}
