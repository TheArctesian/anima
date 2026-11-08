//! What the database tests share: two fixture devices, times on the database's own calendar,
//! and writes and reads of stored rows.

use chrono::{DateTime, Days, NaiveTime, Utc};
use sqlx::PgPool;

use crate::{
    ids::DeviceId,
    metrics::{MetricKind, Sample, record_collection},
};

pub(super) const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
pub(super) const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";

pub(super) fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

pub(super) async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// `time` on the UTC day `days_ago` days before the database's: in the past, and inside the
/// window `metrics.ensure_day_partition` accepts, whatever the hour.
pub(super) async fn day_at(pool: &PgPool, days_ago: u64, time: &str) -> DateTime<Utc> {
    let day = database_now(pool).await.date_naive() - Days::new(days_ago);
    day.and_time(time.parse::<NaiveTime>().unwrap()).and_utc()
}

/// Records `values` (kind, label, value) for `device` taken at `at`.
pub(super) async fn write(
    pool: &PgPool,
    device: DeviceId,
    at: DateTime<Utc>,
    values: &[(MetricKind, &str, f64)],
) {
    let samples: Vec<Sample> = values
        .iter()
        .map(|&(kind, label, value)| Sample::new(kind.into(), label, value).unwrap())
        .collect();
    record_collection(pool, device, at, &samples).await.unwrap();
}

/// How many rows `table` of schema `metrics` has.
pub(super) async fn rows(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM metrics.{table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}
