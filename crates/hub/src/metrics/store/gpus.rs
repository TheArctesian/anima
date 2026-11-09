//! The GPUs found on devices and when the hub last looked (MET-006).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{record::device_missing, rows::GpuRow};
use crate::{
    ids::DeviceId,
    metrics::{Error, Gpu},
};

/// One finished look for a device's GPUs: when, the boot time the device gave then, and what
/// it found.
pub(in crate::metrics) struct Discovery<'a> {
    pub(in crate::metrics) found_at: DateTime<Utc>,
    pub(in crate::metrics) boot_at: Option<DateTime<Utc>>,
    pub(in crate::metrics) gpus: &'a [Gpu],
}

const REPLACE_GPUS_SQL: &str = "\
    INSERT INTO metrics.gpus (device_id, index, name, vendor, has_vram, found_at) \
    SELECT $1, w.index, w.name, w.vendor, w.has_vram, $2 \
    FROM unnest($3::int4[], $4::text[], $5::text[], $6::bool[]) \
        AS w(index, name, vendor, has_vram)";

const DISCOVERY_SQL: &str = "\
    INSERT INTO metrics.discovery (device_id, found_at, boot_at) VALUES ($1, $2, $3) \
    ON CONFLICT (device_id) DO UPDATE \
        SET found_at = excluded.found_at, boot_at = excluded.boot_at";

/// The stored GPUs of each of `devices`, by index, in one query.
pub(in crate::metrics) async fn gpus(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<Gpu>>, Error> {
    let rows: Vec<GpuRow> = sqlx::query_as(
        "SELECT device_id, index, name, vendor, has_vram FROM metrics.gpus \
         WHERE device_id = ANY($1) ORDER BY device_id, index",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    let mut by_device: HashMap<DeviceId, Vec<Gpu>> = HashMap::new();
    for row in rows {
        let device = row.device_id;
        by_device
            .entry(device)
            .or_default()
            .push(Gpu::try_from(row)?);
    }
    Ok(by_device)
}

/// Replaces the stored GPUs of `device` with those of `discovery` and records when it was
/// made, in one transaction, so a reader sees the old set or the new one, never a mix.
pub(in crate::metrics) async fn replace_gpus(
    db: &PgPool,
    device: DeviceId,
    discovery: Discovery<'_>,
) -> Result<(), Error> {
    let gpus = discovery.gpus;
    let indexes: Vec<i32> = gpus.iter().map(|gpu| gpu.index).collect();
    let names: Vec<&str> = gpus.iter().map(|gpu| gpu.name.as_str()).collect();
    let vendors: Vec<&str> = gpus.iter().map(|gpu| gpu.vendor.as_str()).collect();
    let vram: Vec<bool> = gpus.iter().map(|gpu| gpu.has_vram).collect();

    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM metrics.gpus WHERE device_id = $1")
        .bind(device)
        .execute(&mut *tx)
        .await?;
    sqlx::query(REPLACE_GPUS_SQL)
        .bind(device)
        .bind(discovery.found_at)
        .bind(indexes)
        .bind(names)
        .bind(vendors)
        .bind(vram)
        .execute(&mut *tx)
        .await
        .map_err(device_missing)?;
    sqlx::query(DISCOVERY_SQL)
        .bind(device)
        .bind(discovery.found_at)
        .bind(discovery.boot_at)
        .execute(&mut *tx)
        .await
        .map_err(device_missing)?;
    tx.commit().await?;
    Ok(())
}
