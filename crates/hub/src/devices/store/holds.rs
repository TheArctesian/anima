//! SQL that holds a device row for another domain's transaction until it ends, so that
//! transaction takes the device row before its own rows, the order `delete_device` takes them in.

use sqlx::PgConnection;

use crate::{
    devices::{Error, MetricsSource, Missing},
    ids::DeviceId,
};

/// Locks the device row `FOR UPDATE` until the transaction ends.
pub(in crate::devices) async fn lock_device(
    conn: &mut PgConnection,
    id: DeviceId,
) -> Result<(), Error> {
    let row: Option<DeviceId> =
        sqlx::query_scalar("SELECT id FROM devices.devices WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(conn)
            .await?;
    row.map(|_| ()).ok_or(Error::NotFound(Missing::Device(id)))
}

/// Locks the device row `FOR UPDATE` until the transaction ends, and reads its metrics source.
pub(in crate::devices) async fn lock_metrics_source(
    conn: &mut PgConnection,
    id: DeviceId,
) -> Result<MetricsSource, Error> {
    let row: Option<String> =
        sqlx::query_scalar("SELECT metrics_source FROM devices.devices WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(conn)
            .await?;
    let text = row.ok_or(Error::NotFound(Missing::Device(id)))?;
    text.parse()
        .map_err(|error| Error::Database(sqlx::Error::Decode(Box::new(error))))
}

/// Locks the device row `FOR SHARE` until the transaction ends, and reads whether it records
/// its sessions.
pub(in crate::devices) async fn share_record_sessions(
    conn: &mut PgConnection,
    id: DeviceId,
) -> Result<bool, Error> {
    let row: Option<bool> =
        sqlx::query_scalar("SELECT record_sessions FROM devices.devices WHERE id = $1 FOR SHARE")
            .bind(id)
            .fetch_optional(conn)
            .await?;
    row.ok_or(Error::NotFound(Missing::Device(id)))
}
