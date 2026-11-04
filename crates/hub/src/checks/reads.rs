//! Reads of one check, one device's checks, a check's status and uptime, and its heartbeats.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{Check, Error, Heartbeat, UptimeWindow, from_devices, status, store, uptime};
use crate::{
    devices,
    ids::{CheckId, DeviceId},
    maintenance,
    status::Status,
};

/// The device's checks in byte order of the name.
///
/// # Errors
/// `Unreadable` when a stored configuration cannot be read.
pub async fn for_device(db: &PgPool, device: DeviceId) -> Result<Vec<Check>, Error> {
    store::by_devices(db, &[device]).await
}

/// The checks of each device, each device's in byte order of the name, in one query
/// (API-005). A device with no checks is absent.
///
/// # Errors
/// `Unreadable` when a stored configuration cannot be read.
pub async fn for_devices(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<Check>>, Error> {
    store::grouped_by_devices(db, devices).await
}

/// The check with this id.
///
/// # Errors
/// `NotFound` when there is no such check.
pub async fn get(db: &PgPool, id: CheckId) -> Result<Check, Error> {
    store::fetch(db, id).await
}

/// The status the check shows: its stored status with pause, group and maintenance applied
/// (CHK-004, CHK-006). It agrees with [`device_status`] for the check's device.
///
/// # Errors
/// `NotFound` when the device is gone; `Database`, `Devices` or `Maintenance` when a read fails.
pub async fn check_status(db: &PgPool, check: &Check, now: DateTime<Utc>) -> Result<Status, Error> {
    let device = devices::get(db, check.device).await.map_err(from_devices)?;
    let checks = store::standings(db, &[check.device]).await?;
    let active = maintenance::active_targets(db, now)
        .await
        .map_err(Error::Maintenance)?;
    Ok(status::of_check(check.id, &device, &checks, &active))
}

/// The uptime of one check over the window ending at `now`, from the aggregate tables;
/// `None` when it has no counted beats (CHK-011).
///
/// # Errors
/// `Database` when the query fails.
pub async fn check_uptime(
    db: &PgPool,
    check: CheckId,
    window: UptimeWindow,
    now: DateTime<Utc>,
) -> Result<Option<f64>, Error> {
    let span = (window.start(now), now);
    let tally = store::check_tally(db, check, window.resolution(), span).await?;
    Ok(uptime::ratio(tally))
}

/// Up to `limit` heartbeats of the check recorded before `before`, newest first.
///
/// # Errors
/// `Database` when the query fails.
pub async fn heartbeats(
    db: &PgPool,
    check: CheckId,
    before: Option<DateTime<Utc>>,
    limit: u32,
) -> Result<Vec<Heartbeat>, Error> {
    store::heartbeats_before(db, check, before, i64::from(limit)).await
}

/// The PUSH check that holds this token, or `None` (R-028).
///
/// # Errors
/// `Database` when the query fails; `Unreadable` when the stored configuration is not.
pub async fn push_check(db: &PgPool, token: &str) -> Result<Option<Check>, Error> {
    store::by_push_token(db, token).await
}
