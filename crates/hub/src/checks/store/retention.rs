//! Retention and partition upkeep for the daily jobs (DB-005, DB-009, DEV-015).

use chrono::{DateTime, Months, NaiveDate, TimeDelta, Utc};
use sqlx::PgPool;

use crate::checks::Error;

const MINUTE_KEPT: TimeDelta = TimeDelta::days(7);
const HOUR_KEPT: TimeDelta = TimeDelta::days(90);
const DAY_KEPT: Months = Months::new(24);

/// Deletes the heartbeats whose check no longer exists; returns how many (DEV-015).
pub(in crate::checks) async fn delete_orphan_heartbeats(db: &PgPool) -> Result<u64, Error> {
    let deleted = sqlx::query(
        "DELETE FROM checks.heartbeats h \
         WHERE NOT EXISTS (SELECT 1 FROM checks.checks c WHERE c.id = h.check_id)",
    )
    .execute(db)
    .await?;
    Ok(deleted.rows_affected())
}

/// The aggregate rows deleted, per table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Trimmed {
    pub minute: u64,
    pub hour: u64,
    pub day: u64,
}

/// Deletes aggregate buckets older than 7 days (minute), 90 days (hour) and 2 years (day)
/// before `now`, or before the database clock when `now` is later (DB-009, CQ-021). A
/// bucket that starts exactly at the limit is kept.
pub(in crate::checks) async fn trim_aggregates(
    db: &PgPool,
    now: DateTime<Utc>,
) -> Result<Trimmed, Error> {
    let database_now: DateTime<Utc> = sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(db)
        .await?;
    let now = now.min(database_now);
    let day_limit = now.checked_sub_months(DAY_KEPT).unwrap_or(now);
    let delete = async |sql: &'static str, limit: DateTime<Utc>| {
        sqlx::query(sql)
            .bind(limit)
            .execute(db)
            .await
            .map(|done| done.rows_affected())
    };
    Ok(Trimmed {
        minute: delete(
            "DELETE FROM checks.uptime_minute WHERE bucket < $1",
            now - MINUTE_KEPT,
        )
        .await?,
        hour: delete(
            "DELETE FROM checks.uptime_hour WHERE bucket < $1",
            now - HOUR_KEPT,
        )
        .await?,
        day: delete("DELETE FROM checks.uptime_day WHERE bucket < $1", day_limit).await?,
    })
}

/// Drops the heartbeat partitions of the UTC days before `cutoff`; returns their names. The
/// database refuses a cutoff inside the 30-day retention (DB-005).
pub(in crate::checks) async fn drop_heartbeat_days_before(
    db: &PgPool,
    cutoff: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Vec<String>, Error> {
    Ok(
        sqlx::query_scalar("SELECT checks.drop_day_partitions_before($1, $2)")
            .bind(cutoff)
            .bind(now)
            .fetch_all(db)
            .await?,
    )
}

/// Creates the heartbeat partitions of the UTC day of `today` and the `days` days after it,
/// in one statement (DB-004).
pub(in crate::checks) async fn create_days_ahead(
    db: &PgPool,
    today: DateTime<Utc>,
    days: u32,
) -> Result<(), Error> {
    sqlx::query(
        "SELECT checks.ensure_day_partition(day) \
         FROM generate_series($1::timestamptz, $1::timestamptz + make_interval(days => $2), \
                              interval '1 day') AS day",
    )
    .bind(today)
    .bind(i32::try_from(days).unwrap_or(i32::MAX))
    .execute(db)
    .await?;
    Ok(())
}
