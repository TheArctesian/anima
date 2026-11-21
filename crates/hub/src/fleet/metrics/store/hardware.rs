//! The hardware values of each device: replaced whole when a daemon batch brings them
//! (CAP-001 to CAP-005, CAP-032).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use daemon_core::hardware::{BackendReason, Hardware, PowerSource, ThermalState};
use sqlx::{PgConnection, PgPool};

use super::{record::device_missing, rows::HardwareRow};
use crate::{fleet::metrics::Error, ids::DeviceId};

/// The stored hardware values of each device and when the hub stored them; a device with none
/// has no entry. A backend of a name this version does not know is left out.
pub(in crate::fleet::metrics) async fn hardware(
    db: &PgPool,
) -> Result<HashMap<DeviceId, (Hardware, DateTime<Utc>)>, Error> {
    let rows: Vec<HardwareRow> = sqlx::query_as(
        "SELECT device_id, system_memory_bytes, unified_memory_bytes, dedicated_memory_bytes, \
           usable_memory_bytes, usable_memory_source, free_disk_bytes, cpu_features, \
           backend_names, backend_reasons, power_source, thermal_state, operator_active, \
           found_at \
         FROM metrics.hardware",
    )
    .fetch_all(db)
    .await?;
    rows.into_iter().map(HardwareRow::into_entry).collect()
}

/// Replaces the stored hardware values of `device` with `hardware`, found at `found_at`, in the
/// caller's transaction.
pub(in crate::fleet::metrics) async fn write_hardware(
    conn: &mut PgConnection,
    device: DeviceId,
    hardware: &Hardware,
    found_at: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query("DELETE FROM metrics.hardware WHERE device_id = $1")
        .bind(device)
        .execute(&mut *conn)
        .await?;
    let names: Vec<&str> = hardware
        .backends
        .iter()
        .map(|one| one.name.as_str())
        .collect();
    let reasons: Vec<Option<&str>> = hardware
        .backends
        .iter()
        .map(|one| one.state.map(BackendReason::as_str))
        .collect();
    let usable = hardware.usable_memory;
    sqlx::query(
        "INSERT INTO metrics.hardware (device_id, system_memory_bytes, unified_memory_bytes, \
           dedicated_memory_bytes, usable_memory_bytes, usable_memory_source, free_disk_bytes, \
           cpu_features, backend_names, backend_reasons, power_source, thermal_state, \
           operator_active, found_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(device)
    .bind(signed(hardware.system_memory_bytes))
    .bind(hardware.unified_memory_bytes.map(signed))
    .bind(hardware.dedicated_memory_bytes.map(signed))
    .bind(usable.map(|usable| signed(usable.bytes)))
    .bind(usable.map(|usable| usable.source.as_str()))
    .bind(hardware.free_disk_bytes.map(signed))
    .bind(&hardware.cpu_features)
    .bind(&names)
    .bind(&reasons)
    .bind(hardware.power_source.map(PowerSource::as_str))
    .bind(hardware.thermal_state.map(ThermalState::as_str))
    .bind(hardware.operator_active)
    .bind(found_at)
    .execute(&mut *conn)
    .await
    .map_err(device_missing)?;
    Ok(())
}

/// A byte count as a `bigint`: the batch check keeps each at most 2^63 − 1.
fn signed(bytes: u64) -> i64 {
    i64::try_from(bytes).unwrap_or(i64::MAX)
}
