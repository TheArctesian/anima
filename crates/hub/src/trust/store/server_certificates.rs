//! The SQL on `trust.server_certificates`: the RDP server certificate accepted for each
//! device (SEC-007).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::is_violation_of;
use crate::{
    ids::{DeviceId, OperatorId},
    trust::{Error, model::ServerCertificate},
};

#[derive(sqlx::FromRow)]
struct ServerCertificateRow {
    device_id: DeviceId,
    fingerprint_sha256: String,
    subject: Option<String>,
    not_after: Option<DateTime<Utc>>,
    first_seen_at: DateTime<Utc>,
}

impl From<ServerCertificateRow> for (DeviceId, ServerCertificate) {
    fn from(row: ServerCertificateRow) -> Self {
        let certificate = ServerCertificate {
            fingerprint_sha256: row.fingerprint_sha256,
            subject: row.subject,
            not_after: row.not_after,
            first_seen_at: row.first_seen_at,
        };
        (row.device_id, certificate)
    }
}

/// The accepted server certificate of each listed device that has one, in one query
/// (API-005). An empty list returns an empty map without a query.
pub(in crate::trust) async fn server_certificates<'e>(
    db: impl PgExecutor<'e>,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, ServerCertificate>, Error> {
    if devices.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<ServerCertificateRow> = sqlx::query_as(
        "select device_id, fingerprint_sha256, subject, not_after, first_seen_at \
         from trust.server_certificates where device_id = any($1)",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(<(DeviceId, ServerCertificate)>::from)
        .collect())
}

/// Waits for, then holds until the transaction ends, this domain's advisory lock on the
/// device's server certificate, so two accepts for one device run one after the other and each
/// audit entry names the certificate it replaced.
pub(in crate::trust) async fn lock_server_certificate(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<(), Error> {
    sqlx::query(
        "select pg_advisory_xact_lock(\
         hashtextextended('trust.server_certificates:' || $1::text, 0))",
    )
    .bind(device)
    .execute(conn)
    .await?;
    Ok(())
}

/// A server certificate the operator accepts for a device.
pub(in crate::trust) struct AcceptedCertificate<'a> {
    pub(in crate::trust) device: DeviceId,
    pub(in crate::trust) certificate: &'a ServerCertificate,
    pub(in crate::trust) by: OperatorId,
}

/// Stores the accepted certificate, replacing the device's previous one; `NotFound` when the
/// device is gone.
pub(in crate::trust) async fn put_server_certificate(
    conn: &mut PgConnection,
    accepted: &AcceptedCertificate<'_>,
) -> Result<(), Error> {
    let certificate = accepted.certificate;
    let stored = sqlx::query(
        "insert into trust.server_certificates (device_id, fingerprint_sha256, subject, \
                not_after, first_seen_at, accepted_by) \
         values ($1, $2, $3, $4, $5, $6) \
         on conflict (device_id) do update set fingerprint_sha256 = excluded.fingerprint_sha256, \
             subject = excluded.subject, not_after = excluded.not_after, \
             first_seen_at = excluded.first_seen_at, accepted_by = excluded.accepted_by",
    )
    .bind(accepted.device)
    .bind(&certificate.fingerprint_sha256)
    .bind(certificate.subject.as_deref())
    .bind(certificate.not_after)
    .bind(certificate.first_seen_at)
    .bind(accepted.by)
    .execute(conn)
    .await;
    match stored {
        Ok(_) => Ok(()),
        Err(error) if is_violation_of(&error, "server_certificates_device_id_fkey") => {
            Err(Error::NotFound(accepted.device))
        }
        Err(error) => Err(error.into()),
    }
}
