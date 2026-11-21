//! The batteries of each device: replaced together when a batch brings a list (MET-032).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use super::{record::device_missing, rows::BatteryRow};
use crate::{
    fleet::metrics::{Battery, Error},
    ids::DeviceId,
};

const ADD_BATTERIES_SQL: &str = "\
    INSERT INTO metrics.batteries \
        (device_id, name, state, seconds_left, health_percent, found_at) \
    SELECT $1, b.name, b.state, b.seconds_left, b.health_percent, $2 \
    FROM unnest($3::text[], $4::text[], $5::int4[], $6::int2[]) \
        AS b(name, state, seconds_left, health_percent)";

/// The stored batteries of each of `devices`, by name, in one query; a device with none has no
/// entry.
pub(in crate::fleet::metrics) async fn batteries(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<Battery>>, Error> {
    let rows: Vec<BatteryRow> = sqlx::query_as(
        "SELECT device_id, name, state, seconds_left, health_percent FROM metrics.batteries \
         WHERE device_id = ANY($1) ORDER BY device_id, name COLLATE \"C\"",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    let mut by_device: HashMap<DeviceId, Vec<Battery>> = HashMap::new();
    for row in rows {
        let device = row.device_id;
        by_device
            .entry(device)
            .or_default()
            .push(Battery::try_from(row)?);
    }
    Ok(by_device)
}

/// Replaces the stored batteries of `device` with `batteries`, found at `found_at`, in the
/// caller's transaction, so a reader sees the old list or the new one, never a mix.
pub(in crate::fleet::metrics) async fn write_batteries(
    conn: &mut PgConnection,
    device: DeviceId,
    batteries: &[Battery],
    found_at: DateTime<Utc>,
) -> Result<(), Error> {
    let names: Vec<&str> = batteries.iter().map(|one| one.name.as_str()).collect();
    let states: Vec<&str> = batteries
        .iter()
        .map(|one| daemon_core::readings::BatteryState::from(one.state).as_str())
        .collect();
    let seconds: Vec<Option<i32>> = batteries.iter().map(|one| one.seconds_left).collect();
    // The batch check keeps a health within 0 to 100 (K8 Section 4).
    let health: Vec<Option<i16>> = batteries
        .iter()
        .map(|one| {
            one.health_percent
                .and_then(|health| i16::try_from(health).ok())
        })
        .collect();
    sqlx::query("DELETE FROM metrics.batteries WHERE device_id = $1")
        .bind(device)
        .execute(&mut *conn)
        .await?;
    sqlx::query(ADD_BATTERIES_SQL)
        .bind(device)
        .bind(found_at)
        .bind(names)
        .bind(states)
        .bind(seconds)
        .bind(health)
        .execute(conn)
        .await
        .map_err(device_missing)?;
    Ok(())
}
