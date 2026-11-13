//! An agent batch's write: its samples at their own times, and the minute, hour and day rows
//! of each (MET-015, DB-008, K8 Section 4). Every statement runs in the caller's transaction.

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use sqlx::PgConnection;

use crate::{
    fleet::metrics::{Error, Sample},
    ids::DeviceId,
};

/// One value of a pushed point: the mean of its window as a sample at `at`, and the highest
/// sample of the window when the batch had one (CPU and GPU use only).
pub(in crate::fleet::metrics) struct PointValue {
    pub(in crate::fleet::metrics) at: DateTime<Utc>,
    pub(in crate::fleet::metrics) sample: Sample,
    pub(in crate::fleet::metrics) max: Option<f64>,
}

/// The upsert of one aggregate table from the samples just kept: each bucket's new samples are
/// folded first, then merged into the stored row as a mean weighted by count. The new mean is
/// a sum of `value / n`, and the merge moves the stored mean by a share of the difference, so
/// neither overflows for finite values of at least zero, however large (MET-002). The max
/// column takes the window's max where the batch had one, else the mean.
macro_rules! aggregate {
    ($table:literal) => {
        concat!(
            "to_",
            $table,
            " AS (INSERT INTO metrics.",
            $table,
            " AS a (series_id, bucket, avg, min, max, count) \
               SELECT series_id, bucket, sum(value / n), min(value), max(high), count(*)::int4 \
               FROM (SELECT series_id, date_trunc('",
            $table,
            "', at, 'UTC') AS bucket, value, COALESCE(max, value) AS high, \
                       count(*) OVER (PARTITION BY series_id, date_trunc('",
            $table,
            "', at, 'UTC')) AS n FROM kept) folded \
               GROUP BY series_id, bucket \
               ON CONFLICT (series_id, bucket) DO UPDATE SET \
                  avg = a.avg + (excluded.avg - a.avg) \
                        * (excluded.count::float8 / (a.count + excluded.count)), \
                  min = LEAST(a.min, excluded.min), \
                  max = GREATEST(a.max, excluded.max), \
                  count = a.count + excluded.count)"
        )
    };
}

/// A value whose series and time are stored already is skipped, and so is its share of the
/// aggregates, so a batch sent twice adds nothing twice. Returns how many samples it kept.
const ADD_POINTS_SQL: &str = concat!(
    "WITH pushed AS ( \
        SELECT s.id AS series_id, w.at, w.value, w.max \
        FROM unnest($2::text[], $3::text[], $4::timestamptz[], $5::float8[], $6::float8[]) \
            AS w(kind, label, at, value, max) \
        JOIN metrics.series s \
          ON s.device_id = $1 AND s.kind = w.kind AND s.label = w.label), \
     added AS ( \
        INSERT INTO metrics.samples (series_id, at, value) \
        SELECT series_id, at, value FROM pushed \
        ON CONFLICT (series_id, at) DO NOTHING \
        RETURNING series_id, at), \
     kept AS (SELECT pushed.* FROM pushed JOIN added USING (series_id, at)), ",
    aggregate!("minute"),
    ", ",
    aggregate!("hour"),
    ", ",
    aggregate!("day"),
    " SELECT count(*) FROM kept"
);

/// How many series of the device were created at or after `since`, and how many of the
/// series of `samples` it does not have yet, each counted once.
const NEW_SERIES_SQL: &str = "\
    SELECT (SELECT count(*) FROM metrics.series \
            WHERE device_id = $1 AND created_at >= $2) \
         + (SELECT count(*) FROM ( \
                SELECT DISTINCT w.kind, w.label \
                FROM unnest($3::text[], $4::text[]) AS w(kind, label)) w \
            WHERE NOT EXISTS ( \
                SELECT 1 FROM metrics.series s \
                WHERE s.device_id = $1 AND s.kind = w.kind AND s.label = w.label))";

/// How many series `device` will have created at or after `since` once the series of `samples`
/// it lacks are added (R-098 e, CR-20). The creation time is the hub's, so a batch cannot move
/// a series out of the count by the times of its points.
pub(in crate::fleet::metrics) async fn created_series(
    conn: &mut PgConnection,
    device: DeviceId,
    since: DateTime<Utc>,
    samples: &[Sample],
) -> Result<i64, Error> {
    let kinds: Vec<&str> = samples.iter().map(|sample| sample.kind.as_str()).collect();
    let labels: Vec<&str> = samples.iter().map(|sample| sample.label.as_str()).collect();
    let count = sqlx::query_scalar(NEW_SERIES_SQL)
        .bind(device)
        .bind(since)
        .bind(kinds)
        .bind(labels)
        .fetch_one(conn)
        .await?;
    Ok(count)
}

/// Writes `values` of `device`, whose series exist, with their aggregate rows, after creating
/// the sample partition of each UTC day they fall on; how many samples were new.
pub(in crate::fleet::metrics) async fn add_points(
    conn: &mut PgConnection,
    device: DeviceId,
    values: &[PointValue],
) -> Result<i64, Error> {
    let days: BTreeSet<NaiveDate> = values.iter().map(|value| value.at.date_naive()).collect();
    for day in days {
        sqlx::query("SELECT metrics.ensure_day_partition($1)")
            .bind(day.and_time(NaiveTime::MIN).and_utc())
            .execute(&mut *conn)
            .await?;
    }
    let kinds: Vec<&str> = values.iter().map(|one| one.sample.kind.as_str()).collect();
    let labels: Vec<&str> = values.iter().map(|one| one.sample.label.as_str()).collect();
    let ats: Vec<DateTime<Utc>> = values.iter().map(|one| one.at).collect();
    let means: Vec<f64> = values.iter().map(|one| one.sample.value).collect();
    let maxes: Vec<Option<f64>> = values.iter().map(|one| one.max).collect();
    let kept = sqlx::query_scalar(ADD_POINTS_SQL)
        .bind(device)
        .bind(kinds)
        .bind(labels)
        .bind(ats)
        .bind(means)
        .bind(maxes)
        .fetch_one(conn)
        .await?;
    Ok(kept)
}
