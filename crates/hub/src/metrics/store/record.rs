//! One collection's write: series, samples and the three aggregate rows of each (DB-008).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    ids::{DeviceId, MetricSeriesId},
    metrics::{Error, Sample},
};

const UPSERT_SERIES_SQL: &str = "\
    INSERT INTO metrics.series (id, device_id, kind, label) \
    SELECT w.id, $1, w.kind, w.label \
    FROM unnest($2::uuid[], $3::text[], $4::text[]) AS w(id, kind, label) \
    ON CONFLICT (device_id, kind, label) DO NOTHING";

/// The upsert of one aggregate table from the samples just inserted: the mean is kept as a
/// running mean over `count` samples, and the bucket starts on the UTC minute, hour or day, as the table is named.
macro_rules! aggregate {
    ($table:literal) => {
        concat!(
            "to_",
            $table,
            " AS (INSERT INTO metrics.",
            $table,
            " AS a (series_id, bucket, avg, min, max, count) \
               SELECT series_id, date_trunc('",
            $table,
            "', $2::timestamptz, 'UTC'), value, value, value, 1 FROM added \
               ON CONFLICT (series_id, bucket) DO UPDATE SET \
                  avg = (a.avg * a.count + excluded.avg) / (a.count + 1), \
                  min = LEAST(a.min, excluded.min), \
                  max = GREATEST(a.max, excluded.max), \
                  count = a.count + 1)"
        )
    };
}

/// A sample whose (series, time) exists is skipped, and so is its aggregate update, so a
/// repeated write adds nothing twice.
const ADD_SAMPLES_SQL: &str = concat!(
    "WITH added AS ( \
        INSERT INTO metrics.samples (series_id, at, value) \
        SELECT s.id, $2::timestamptz, w.value \
        FROM unnest($3::text[], $4::text[], $5::float8[]) AS w(kind, label, value) \
        JOIN metrics.series s \
          ON s.device_id = $1 AND s.kind = w.kind AND s.label = w.label \
        ON CONFLICT (series_id, at) DO NOTHING \
        RETURNING series_id, value), ",
    aggregate!("minute"),
    ", ",
    aggregate!("hour"),
    ", ",
    aggregate!("day"),
    " SELECT count(*) FROM added"
);

/// Writes `samples` taken at `at` for `device` in one transaction: the series rows they need,
/// the samples, and the minute, hour and day rows. Three statements however many samples.
pub(in crate::metrics) async fn record(
    db: &PgPool,
    device: DeviceId,
    at: DateTime<Utc>,
    samples: &[Sample],
) -> Result<(), Error> {
    sqlx::query("SELECT metrics.ensure_day_partition($1)")
        .bind(at)
        .execute(db)
        .await?;
    let ids: Vec<MetricSeriesId> = samples
        .iter()
        .map(|_| MetricSeriesId::from(Uuid::now_v7()))
        .collect();
    let kinds: Vec<&str> = samples.iter().map(|sample| sample.kind.as_str()).collect();
    let labels: Vec<&str> = samples.iter().map(|sample| sample.label.as_str()).collect();
    let values: Vec<f64> = samples.iter().map(|sample| sample.value).collect();

    let mut tx = db.begin().await?;
    sqlx::query(UPSERT_SERIES_SQL)
        .bind(device)
        .bind(ids)
        .bind(&kinds)
        .bind(&labels)
        .execute(&mut *tx)
        .await
        .map_err(device_missing)?;
    sqlx::query(ADD_SAMPLES_SQL)
        .bind(device)
        .bind(at)
        .bind(&kinds)
        .bind(&labels)
        .bind(values)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// A foreign key violation on the series table means the device is gone.
fn device_missing(error: sqlx::Error) -> Error {
    match error.as_database_error() {
        Some(db_error) if db_error.is_foreign_key_violation() => Error::NoDevice,
        _ => Error::Database(error),
    }
}
