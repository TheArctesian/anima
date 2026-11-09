//! An agent batch's write: its samples at their own times, and the minute, hour and day rows
//! of each (MET-015, DB-008, K8 Section 4). Every statement runs in the caller's transaction.

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use sqlx::PgConnection;

use crate::{
    ids::DeviceId,
    metrics::{Error, Sample},
};

/// One value of a pushed point: the mean of its window as a sample at `at`, and the highest
/// sample of the window when the batch had one (CPU and GPU use only).
pub(in crate::metrics) struct PointValue {
    pub(in crate::metrics) at: DateTime<Utc>,
    pub(in crate::metrics) sample: Sample,
    pub(in crate::metrics) max: Option<f64>,
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

/// How many series of the device had a sample at or after `since`, counted together with the
/// series of `samples`, each once.
const LIVE_SERIES_SQL: &str = "\
    SELECT count(*) FROM ( \
        SELECT s.kind, s.label FROM metrics.series s \
        WHERE s.device_id = $1 AND EXISTS ( \
            SELECT 1 FROM metrics.samples m WHERE m.series_id = s.id AND m.at >= $2) \
        UNION \
        SELECT w.kind, w.label FROM unnest($3::text[], $4::text[]) AS w(kind, label)) live";

/// How many series `device` would have with a sample at or after `since` once the series of
/// `samples` are added (R-098 e).
pub(in crate::metrics) async fn live_series(
    conn: &mut PgConnection,
    device: DeviceId,
    since: DateTime<Utc>,
    samples: &[Sample],
) -> Result<i64, Error> {
    let kinds: Vec<&str> = samples.iter().map(|sample| sample.kind.as_str()).collect();
    let labels: Vec<&str> = samples.iter().map(|sample| sample.label.as_str()).collect();
    let count = sqlx::query_scalar(LIVE_SERIES_SQL)
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
pub(in crate::metrics) async fn add_points(
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
