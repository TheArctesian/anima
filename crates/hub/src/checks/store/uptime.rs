//! Uptime sums, read from the aggregate tables only, never from raw heartbeats (CHK-011).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    checks::{
        Error,
        uptime::{Resolution, Tally},
    },
    ids::DeviceId,
};

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
