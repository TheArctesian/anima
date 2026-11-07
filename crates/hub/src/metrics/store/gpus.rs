//! The GPUs stored for devices (MET-006).

use std::collections::HashMap;

use sqlx::PgPool;

use super::rows::GpuRow;
use crate::{
    ids::DeviceId,
    metrics::{Error, Gpu},
};

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
