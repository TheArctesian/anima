//! Retention and partition upkeep for the daily jobs (DB-005, DB-009, DEV-015).

use chrono::{DateTime, Days, Months, NaiveDate, TimeDelta, Utc};
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

/// Deletes aggregate buckets older than 7 days (minute), 90 days (hour) and 2 years (day)
/// before `now`, or before the database clock when `now` is later (DB-009, CQ-021). A
/// bucket that starts exactly at the limit is kept.
pub(in crate::checks) async fn trim_aggregates(
    db: &PgPool,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let database_now: DateTime<Utc> = sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(db)
        .await?;
    let now = now.min(database_now);
    let day_limit = now.checked_sub_months(DAY_KEPT).unwrap_or(now);
    let delete = async |sql: &'static str, limit: DateTime<Utc>| {
        sqlx::query(sql).bind(limit).execute(db).await
    };
    delete(
        "DELETE FROM checks.uptime_minute WHERE bucket < $1",
        now - MINUTE_KEPT,
    )
    .await?;
    delete(
        "DELETE FROM checks.uptime_hour WHERE bucket < $1",
        now - HOUR_KEPT,
    )
    .await?;
    delete("DELETE FROM checks.uptime_day WHERE bucket < $1", day_limit).await?;
    Ok(())
}

/// The first UTC day whose heartbeats are kept at `now` when they are kept `keep`. It is one
/// day later than the database allows, so a hub clock slightly ahead of the database clock is
/// never refused; the extra day is dropped on the next run.
fn first_kept_day(now: DateTime<Utc>, keep: Days) -> NaiveDate {
    now.naive_utc()
        .checked_sub_days(keep)
        .and_then(|bound| bound.date().pred_opt())
        .unwrap_or(NaiveDate::MIN)
}

/// Drops the heartbeat partitions older than `keep` at `now`; returns their names. The
/// database refuses a cutoff inside its 1-day floor (DB-005).
pub(in crate::checks) async fn drop_heartbeat_days_before(
    db: &PgPool,
    now: DateTime<Utc>,
    keep: Days,
) -> Result<Vec<String>, Error> {
    Ok(
        sqlx::query_scalar("SELECT checks.drop_day_partitions_before($1, $2)")
            .bind(first_kept_day(now, keep))
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

#[cfg(test)]
mod tests {
    use chrono::{Days, NaiveDate, TimeZone, Utc};

    use super::first_kept_day;

    #[test]
    fn db_005_heartbeats_keep_their_period_plus_a_day_of_clock_margin() {
        let now = Utc.with_ymd_and_hms(2028, 3, 1, 0, 30, 0).unwrap();
        let day = |month, day| NaiveDate::from_ymd_opt(2028, month, day).unwrap();
        assert_eq!(first_kept_day(now, Days::new(30)), day(1, 30));
        assert_eq!(first_kept_day(now, Days::new(1)), day(2, 28));
    }
}
