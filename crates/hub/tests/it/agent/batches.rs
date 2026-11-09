//! K8 Section 4 batches for the push tests, and the stored rows they are checked against.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

/// `at` as K8 writes a time.
pub fn time(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// A point ending at `at` with `mean`, one entry per series.
pub fn point(at: DateTime<Utc>, mean: Value) -> Value {
    let mut point = json!({ "at": time(at) });
    point["mean"] = mean;
    point
}

/// A point ending at `at` with `mean` and `max`.
pub fn point_with_max(at: DateTime<Utc>, mean: Value, max: Value) -> Value {
    let mut point = point(at, mean);
    point["max"] = max;
    point
}

/// A batch of `device` with `series` and `points`.
pub fn batch(device: &str, series: Value, points: Vec<Value>) -> Value {
    let mut batch = json!({ "device": device });
    batch["series"] = series;
    batch["points"] = Value::Array(points);
    batch
}

/// A batch of `device` with one CPU point at `at`.
pub fn cpu_batch(device: &str, at: DateTime<Utc>, cpu: f64) -> Value {
    batch(
        device,
        json!([{ "kind": "cpu_percent" }]),
        vec![point(at, json!([cpu]))],
    )
}

/// How many samples the device's series hold.
pub async fn samples_of(pool: &PgPool, device: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM metrics.samples m JOIN metrics.series s ON s.id = m.series_id \
         WHERE s.device_id = $1::uuid",
    )
    .bind(device)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// The device's series as (kind, label, GPU index), in that order.
pub async fn series_of(pool: &PgPool, device: &str) -> Vec<(String, String, Option<i32>)> {
    sqlx::query_as(
        "SELECT kind, label, gpu_index FROM metrics.series WHERE device_id = $1::uuid \
         ORDER BY kind, label",
    )
    .bind(device)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// The (avg, min, max, count) rows of `table` for the device's series of `kind`, by bucket.
pub async fn aggregates(
    pool: &PgPool,
    table: &str,
    device: &str,
    kind: &str,
) -> Vec<(f64, f64, f64, i32)> {
    let sql = format!(
        "SELECT a.avg, a.min, a.max, a.count FROM metrics.{table} a \
         JOIN metrics.series s ON s.id = a.series_id \
         WHERE s.device_id = $1::uuid AND s.kind = $2 ORDER BY a.bucket"
    );
    sqlx::query_as(&sql)
        .bind(device)
        .bind(kind)
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The indexes of the device's stored GPUs.
pub async fn gpus_of(pool: &PgPool, device: &str) -> Vec<i32> {
    sqlx::query_scalar("SELECT index FROM metrics.gpus WHERE device_id = $1::uuid ORDER BY index")
        .bind(device)
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The device name and message of each warning from metrics, oldest first.
pub async fn warnings(pool: &PgPool) -> Vec<(Option<String>, String)> {
    sqlx::query_as(
        "SELECT device_name, message FROM events.event_log \
         WHERE source = 'metrics' AND severity = 'warning' ORDER BY at, message",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}
