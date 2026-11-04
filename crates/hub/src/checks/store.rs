//! All SQL on schema `checks`: I/O only; rows are mapped in `rows`. Ids and time come in as
//! parameters.

/// Every column of a check, in the order `CheckRow` reads them.
macro_rules! check_columns {
    () => {
        "id, device_id, parent_id, type, name, interval_seconds, retries, \
         retry_interval_seconds, degraded_threshold_ms, invert, paused, config, created_at, \
         updated_at, status, failures, next_due_at, last_beat_at, cert_not_after, resolved_ip, \
         push_token"
    };
}

/// A check query with `$tail` after the `FROM`.
macro_rules! select_check {
    ($($tail:expr),*) => {
        concat!("SELECT ", check_columns!(), " FROM checks.checks ", $($tail),*)
    };
}

mod beats;
mod retention;
mod rows;
mod sealed;
mod uptime;
mod writes;

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool};

use super::{
    Error,
    model::{Check, Due, Missing},
    status::Standing,
};
use crate::ids::{CheckId, DeviceId};

pub(super) use beats::record_beat;
pub use retention::Trimmed;
pub(super) use retention::{
    create_days_ahead, delete_orphan_heartbeats, drop_heartbeat_days_before, trim_aggregates,
};
use rows::{CheckRow, StandingRow, to_checks};
pub(super) use sealed::seal;
pub(super) use uptime::tallies;
pub(super) use writes::{
    Change, Current, Insert, delete, insert, load_http_secrets, lock, set_next_due, set_paused,
    update,
};

pub(super) async fn fetch<'e>(db: impl PgExecutor<'e>, id: CheckId) -> Result<Check, Error> {
    let row = sqlx::query_as::<_, CheckRow>(select_check!("WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound(Missing::Check(id)))?;
    Check::try_from(row)
}

/// The checks of the devices, each device's in byte order of the name.
pub(super) async fn by_devices(db: &PgPool, devices: &[DeviceId]) -> Result<Vec<Check>, Error> {
    let rows = sqlx::query_as(select_check!(
        "WHERE device_id = ANY($1) ORDER BY device_id, name COLLATE \"C\", id"
    ))
    .bind(devices)
    .fetch_all(db)
    .await?;
    to_checks(rows)
}

/// What the status rules need about every check of the devices, in one query.
pub(super) async fn standings(db: &PgPool, devices: &[DeviceId]) -> Result<Vec<Standing>, Error> {
    let rows: Vec<StandingRow> = sqlx::query_as(
        "SELECT id, device_id, parent_id, type, paused, status FROM checks.checks \
         WHERE device_id = ANY($1)",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Standing::try_from).collect()
}

/// For each device, the address of the check with the latest beat that has one.
pub(super) async fn resolved_ips(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, IpAddr>, Error> {
    let rows: Vec<(DeviceId, IpAddr)> = sqlx::query_as(
        "SELECT DISTINCT ON (device_id) device_id, resolved_ip FROM checks.checks \
         WHERE device_id = ANY($1) AND resolved_ip IS NOT NULL AND last_beat_at IS NOT NULL \
         ORDER BY device_id, last_beat_at DESC, id",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Checks that are not paused and are due at `now`, the longest-waiting first. Groups are
/// never probed, so they are never due.
pub(super) async fn due(db: &PgPool, now: DateTime<Utc>, limit: u32) -> Result<Vec<Due>, Error> {
    let rows: Vec<CheckRow> = sqlx::query_as(select_check!(
        "WHERE NOT paused AND type <> 'group' AND (next_due_at IS NULL OR next_due_at <= $1) \
         ORDER BY next_due_at NULLS FIRST, id LIMIT $2"
    ))
    .bind(now)
    .bind(i64::from(limit))
    .fetch_all(db)
    .await?;
    rows.into_iter().map(CheckRow::into_due).collect()
}
