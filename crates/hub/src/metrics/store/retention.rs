//! Retention and partition upkeep for the daily job (DB-004, DB-005, DB-009, DEV-015).

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::metrics::{Error, retention::Cutoffs};

/// The database clock, which bounds every "now" a caller passes (CQ-021).
pub(in crate::metrics) async fn database_now(db: &PgPool) -> Result<DateTime<Utc>, Error> {
    Ok(sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(db)
        .await?)
}

/// Creates the sample partitions of the UTC days from `first` through `days` days after it, in
/// one statement (DB-004).
pub(in crate::metrics) async fn create_days_ahead(
    db: &PgPool,
    first: DateTime<Utc>,
    days: i32,
) -> Result<(), Error> {
    sqlx::query(
        "SELECT metrics.ensure_day_partition(day) \
         FROM generate_series($1::timestamptz, $1::timestamptz + make_interval(days => $2), \
                              interval '1 day') AS day",
    )
    .bind(first)
    .bind(days)
    .execute(db)
    .await?;
    Ok(())
}

/// Drops the sample partitions of the UTC days before `cutoff`; returns their names. The
/// database refuses a cutoff inside the 30-day retention (DB-005).
pub(in crate::metrics) async fn drop_days_before(
    db: &PgPool,
    cutoff: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Vec<String>, Error> {
    Ok(
        sqlx::query_scalar("SELECT metrics.drop_day_partitions_before($1, $2)")
            .bind(cutoff)
            .bind(now)
            .fetch_all(db)
            .await?,
    )
}

/// Deletes the minute, hour and day buckets that start before their cutoffs; returns how many
/// of each (DB-009). A bucket that starts exactly at the limit is kept.
pub(in crate::metrics) async fn trim_aggregates(
    db: &PgPool,
    cutoffs: &Cutoffs,
) -> Result<[u64; 3], Error> {
    let delete = async |sql: &'static str, limit: DateTime<Utc>| {
        sqlx::query(sql)
            .bind(limit)
            .execute(db)
            .await
            .map(|done| done.rows_affected())
    };
    Ok([
        delete(
            "DELETE FROM metrics.minute WHERE bucket < $1",
            cutoffs.minute,
        )
        .await?,
        delete("DELETE FROM metrics.hour WHERE bucket < $1", cutoffs.hour).await?,
        delete("DELETE FROM metrics.day WHERE bucket < $1", cutoffs.day).await?,
    ])
}

/// Deletes the samples whose series no longer exists; returns how many (DEV-015).
pub(in crate::metrics) async fn delete_orphan_samples(db: &PgPool) -> Result<u64, Error> {
    let deleted = sqlx::query(
        "DELETE FROM metrics.samples m \
         WHERE NOT EXISTS (SELECT 1 FROM metrics.series s WHERE s.id = m.series_id)",
    )
    .execute(db)
    .await?;
    Ok(deleted.rows_affected())
}

/// Deletes the series that have no minute, hour or day row and no sample since `since`; returns
/// how many (D-5: a device that renames its mounts cannot grow the table without bound). A
/// series a collection writes has a sample from the last minute, so it is never idle; one that
/// is written again later is created again by the upsert of `record`.
pub(in crate::metrics) async fn delete_idle_series(
    db: &PgPool,
    since: DateTime<Utc>,
) -> Result<u64, Error> {
    let deleted = sqlx::query(
        "DELETE FROM metrics.series s \
         WHERE NOT EXISTS (SELECT 1 FROM metrics.minute a WHERE a.series_id = s.id) \
           AND NOT EXISTS (SELECT 1 FROM metrics.hour a WHERE a.series_id = s.id) \
           AND NOT EXISTS (SELECT 1 FROM metrics.day a WHERE a.series_id = s.id) \
           AND NOT EXISTS (SELECT 1 FROM metrics.samples m \
                           WHERE m.series_id = s.id AND m.at >= $1)",
    )
    .bind(since)
    .execute(db)
    .await?;
    Ok(deleted.rows_affected())
}
