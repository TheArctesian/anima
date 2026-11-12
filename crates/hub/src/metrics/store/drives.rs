//! The drives of each device: replaced together when a collection or a batch brings a new
//! list (MET-022).

use std::collections::HashMap;

use agent_core::readings::Drive;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use super::{record::device_missing, rows::DriveRow};
use crate::{ids::DeviceId, metrics::Error};

const ADD_DRIVE_SQL: &str = "\
    INSERT INTO metrics.drives (device_id, name, kind, size_bytes, mounts, found_at) \
    VALUES ($1, $2, $3, $4, $5, $6)";

/// The stored drives of each of `devices`, by name, in one query; a device with none has no
/// entry.
pub(in crate::metrics) async fn drives(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<Drive>>, Error> {
    let rows: Vec<DriveRow> = sqlx::query_as(
        "SELECT device_id, name, kind, size_bytes, mounts FROM metrics.drives \
         WHERE device_id = ANY($1) ORDER BY device_id, name COLLATE \"C\"",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    let mut by_device: HashMap<DeviceId, Vec<Drive>> = HashMap::new();
    for row in rows {
        let device = row.device_id;
        by_device
            .entry(device)
            .or_default()
            .push(Drive::try_from(row)?);
    }
    Ok(by_device)
}

/// Replaces the stored drives of `device` with `drives`, found at `found_at`, in one
/// transaction, so a reader sees the old list or the new one, never a mix.
pub(in crate::metrics) async fn replace_drives(
    db: &PgPool,
    device: DeviceId,
    drives: &[Drive],
    found_at: DateTime<Utc>,
) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    write_drives(&mut tx, device, drives, found_at).await?;
    tx.commit().await?;
    Ok(())
}

/// [`replace_drives`] in the caller's transaction: at most 32 inserts, one per drive.
pub(in crate::metrics) async fn write_drives(
    conn: &mut PgConnection,
    device: DeviceId,
    drives: &[Drive],
    found_at: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query("DELETE FROM metrics.drives WHERE device_id = $1")
        .bind(device)
        .execute(&mut *conn)
        .await?;
    for drive in drives {
        // The readers and the batch check keep a size within i64 (MET-022, K8 Section 4).
        let size = i64::try_from(drive.size_bytes).unwrap_or(i64::MAX);
        sqlx::query(ADD_DRIVE_SQL)
            .bind(device)
            .bind(&drive.name)
            .bind(drive.kind.as_str())
            .bind(size)
            .bind(&drive.mounts)
            .bind(found_at)
            .execute(&mut *conn)
            .await
            .map_err(device_missing)?;
    }
    Ok(())
}
