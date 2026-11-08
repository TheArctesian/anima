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

mod attempts;
mod beats;
mod heartbeats;
pub(super) mod repairs;
mod retention;
mod rows;
mod sealed;
mod snmp_sealed;
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

pub(super) use attempts::record_attempt;
pub(super) use beats::record_beat;
#[cfg(test)]
pub(in crate::checks) use heartbeats::{LATEST_SQL, PAGES_SQL};
pub(super) use heartbeats::{latest as latest_heartbeats, pages as heartbeat_pages};
pub(super) use retention::{
    create_days_ahead, delete_orphan_heartbeats, drop_heartbeat_days_before, trim_aggregates,
};
use rows::{CheckRow, StandingRow, to_checks};
pub(super) use sealed::seal;
#[cfg(test)]
pub(in crate::checks) use snmp_sealed::open as open_snmp;
pub(super) use snmp_sealed::seal as seal_snmp;
pub(super) use uptime::{check_tallies, tallies};
pub(super) use writes::{
    Change, Current, Insert, delete, insert, load_http_secrets, load_snmp_secrets, lock,
    set_next_due, set_paused, update,
};

pub(super) async fn fetch<'e>(db: impl PgExecutor<'e>, id: CheckId) -> Result<Check, Error> {
    let row = sqlx::query_as::<_, CheckRow>(select_check!("WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound(Missing::Check(id)))?;
    Check::try_from(row)
}

/// The checks with these ids that exist, in one query.
pub(super) async fn fetch_many(db: &PgPool, ids: &[CheckId]) -> Result<Vec<Check>, Error> {
    let rows = sqlx::query_as(select_check!("WHERE id = ANY($1)"))
        .bind(ids)
        .fetch_all(db)
        .await?;
    to_checks(rows)
}

/// The check as it is now, its row locked until the caller's transaction ends, so what an audit
/// entry compares against cannot change underneath it.
pub(super) async fn fetch_for_update(
    conn: &mut sqlx::PgConnection,
    id: CheckId,
) -> Result<Check, Error> {
    let row = sqlx::query_as::<_, CheckRow>(select_check!("WHERE id = $1 FOR UPDATE"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(Error::NotFound(Missing::Check(id)))?;
    Check::try_from(row)
}

/// The children of a group, their rows locked until the caller's transaction ends: a child
/// that another transaction is moving is waited for, so the answer is what was committed.
pub(super) async fn children_for_update(
    conn: &mut sqlx::PgConnection,
    group: CheckId,
) -> Result<Vec<CheckId>, Error> {
    let ids = sqlx::query_scalar(
        "SELECT id FROM checks.checks WHERE parent_id = $1 ORDER BY id FOR UPDATE",
    )
    .bind(group)
    .fetch_all(conn)
    .await?;
    Ok(ids)
}

/// The checks of the devices grouped by device, each group in name order.
pub(super) async fn grouped_by_devices(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<Check>>, Error> {
    let mut grouped: HashMap<DeviceId, Vec<Check>> = HashMap::new();
    for check in by_devices(db, devices).await? {
        grouped.entry(check.device).or_default().push(check);
    }
    Ok(grouped)
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

/// The id and URL of every HTTP check of the devices, by device, name (byte order) and id.
/// Reads only the `url` field of the stored JSON, so no other check's config is parsed.
pub(super) async fn http_urls(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<(CheckId, String)>>, sqlx::Error> {
    let rows: Vec<(DeviceId, CheckId, String)> = sqlx::query_as(
        "SELECT device_id, id, config ->> 'url' FROM checks.checks \
         WHERE device_id = ANY($1) AND type = 'http' AND config ->> 'url' IS NOT NULL \
         ORDER BY device_id, name COLLATE \"C\", id",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    let mut grouped: HashMap<DeviceId, Vec<(CheckId, String)>> = HashMap::new();
    for (device, check, url) in rows {
        grouped.entry(device).or_default().push((check, url));
    }
    Ok(grouped)
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

/// The device of each of the checks, in one query.
pub(super) async fn devices_of(
    db: &PgPool,
    checks: &[CheckId],
) -> Result<Vec<(CheckId, DeviceId)>, Error> {
    let rows = sqlx::query_as("SELECT id, device_id FROM checks.checks WHERE id = ANY($1)")
        .bind(checks)
        .fetch_all(db)
        .await?;
    Ok(rows)
}

/// The PUSH check that holds this token, if any.
pub(super) async fn by_push_token(db: &PgPool, token: &str) -> Result<Option<Check>, Error> {
    let row = sqlx::query_as::<_, CheckRow>(select_check!("WHERE push_token = $1"))
        .bind(token)
        .fetch_optional(db)
        .await?;
    row.map(Check::try_from).transpose()
}

/// For each device, the address of the ICMP, TCP or TLS check with the latest beat that has
/// one. Only those probe the device's own host; an HTTP or DNS check can name another.
pub(super) async fn resolved_ips(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, IpAddr>, Error> {
    let rows: Vec<(DeviceId, IpAddr)> = sqlx::query_as(
        "SELECT DISTINCT ON (device_id) device_id, resolved_ip FROM checks.checks \
         WHERE device_id = ANY($1) AND type IN ('icmp', 'tcp', 'tls') \
         AND resolved_ip IS NOT NULL AND last_beat_at IS NOT NULL \
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

/// The devices with a check whose last beat is at or after `since`, from any writer: the
/// scheduler or the push route.
pub(super) async fn devices_beaten_since(
    db: &PgPool,
    since: DateTime<Utc>,
) -> Result<Vec<DeviceId>, Error> {
    Ok(
        sqlx::query_scalar("SELECT DISTINCT device_id FROM checks.checks WHERE last_beat_at >= $1")
            .bind(since)
            .fetch_all(db)
            .await?,
    )
}
