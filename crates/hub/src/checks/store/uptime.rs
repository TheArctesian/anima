//! Uptime sums, read from the aggregate tables only, never from raw heartbeats (CHK-011).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    checks::{
        Error,
        uptime::{Resolution, Tally},
    },
    ids::{CheckId, DeviceId},
};

macro_rules! check_tally {
    ($table:literal) => {
        concat!(
            "SELECT COALESCE(sum(up_count), 0)::bigint, COALESCE(sum(down_count), 0)::bigint, \
                    COALESCE(sum(degraded_count), 0)::bigint \
             FROM checks.",
            $table,
            " WHERE check_id = $1 AND bucket >= $2 AND bucket < $3"
        )
    };
}

const fn check_tally_sql(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::Minute => check_tally!("uptime_minute"),
        Resolution::Hour => check_tally!("uptime_hour"),
        Resolution::Day => check_tally!("uptime_day"),
    }
}

macro_rules! check_tallies {
    ($table:literal) => {
        concat!(
            "SELECT check_id, sum(up_count)::bigint, sum(down_count)::bigint, \
                    sum(degraded_count)::bigint \
             FROM checks.",
            $table,
            " WHERE check_id = ANY($1) AND bucket >= $2 AND bucket < $3 GROUP BY check_id"
        )
    };
}

const fn check_tallies_sql(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::Minute => check_tallies!("uptime_minute"),
        Resolution::Hour => check_tallies!("uptime_hour"),
        Resolution::Day => check_tallies!("uptime_day"),
    }
}

/// The beats of each of the checks in `[start, end)`, from one aggregate table, in one query.
pub(in crate::checks) async fn check_tallies(
    db: &PgPool,
    checks: &[CheckId],
    resolution: Resolution,
    (start, end): (DateTime<Utc>, DateTime<Utc>),
) -> Result<Vec<(CheckId, Tally)>, Error> {
    let rows: Vec<(CheckId, i64, i64, i64)> = sqlx::query_as(check_tallies_sql(resolution))
        .bind(checks)
        .bind(start)
        .bind(end)
        .fetch_all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(check, up, down, degraded)| (check, Tally { up, down, degraded }))
        .collect())
}

/// The beats of one check in `[start, end)`, from one aggregate table.
pub(in crate::checks) async fn check_tally(
    db: &PgPool,
    check: CheckId,
    resolution: Resolution,
    (start, end): (DateTime<Utc>, DateTime<Utc>),
) -> Result<Tally, Error> {
    let (up, down, degraded): (i64, i64, i64) = sqlx::query_as(check_tally_sql(resolution))
        .bind(check)
        .bind(start)
        .bind(end)
        .fetch_one(db)
        .await?;
    Ok(Tally { up, down, degraded })
}

/// Up, down and degraded beats per device over the non-group checks, for the buckets that
/// start in `[$2, $3)`.
macro_rules! tally {
    ($table:literal) => {
        concat!(
            "SELECT c.device_id, sum(u.up_count)::bigint, sum(u.down_count)::bigint, \
                    sum(u.degraded_count)::bigint \
             FROM checks.",
            $table,
            " u JOIN checks.checks c ON c.id = u.check_id \
             WHERE c.device_id = ANY($1) AND c.type <> 'group' \
               AND u.bucket >= $2 AND u.bucket < $3 \
             GROUP BY c.device_id"
        )
    };
}

const fn tally_sql(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::Minute => tally!("uptime_minute"),
        Resolution::Hour => tally!("uptime_hour"),
        Resolution::Day => tally!("uptime_day"),
    }
}

/// The beats of each device's non-group checks in `[start, end)`, from one aggregate table.
pub(in crate::checks) async fn tallies(
    db: &PgPool,
    devices: &[DeviceId],
    resolution: Resolution,
    (start, end): (DateTime<Utc>, DateTime<Utc>),
) -> Result<Vec<(DeviceId, Tally)>, Error> {
    let rows: Vec<(DeviceId, i64, i64, i64)> = sqlx::query_as(tally_sql(resolution))
        .bind(devices)
        .bind(start)
        .bind(end)
        .fetch_all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(device, up, down, degraded)| (device, Tally { up, down, degraded }))
        .collect())
}
