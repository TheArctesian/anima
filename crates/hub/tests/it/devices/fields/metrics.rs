//! `Device.readings`, `Device.metrics` and `Device.gpus` (MET-003, MET-004, MET-006, API-005).

use std::sync::{Arc, atomic::Ordering};

use anima_hub::{
    clock::{Clock, FixedClock},
    devices::graphql::{Mutation, Query, gpus_loader, readings_loader},
    metrics::{MetricKind, Sample, record_collection},
};
use async_graphql::{EmptySubscription, Schema};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{PVE_01, names};
use crate::devices::{
    batching::{BATCH_WINDOW, SITES, counting_pool},
    graphql_read::add_bulk,
    harness::{data, schema_with_clock},
};

const DETAIL: &str = "{ device(id: \"0190a000-0000-7000-8002-000000000001\") { \
    readings { at cpuPercent memoryPercent diskPercent vramPercent vramGpu { name } } \
    metrics(range: HOUR) { kind label gpuIndex points { at value min max } } \
    gpus { index name vendor hasVram } } }";

const LIST: &str = "{ devices(first: 200) { edges { node { name readings { cpuPercent } \
    gpus { name } } } } }";

async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Records a CPU and a memory sample for `device` taken at `at`.
async fn record(pool: &PgPool, device: &str, at: DateTime<Utc>, cpu: f64) {
    let samples = [
        Sample::new(MetricKind::CpuPercent.into(), "", cpu).unwrap(),
        Sample::new(MetricKind::MemoryPercent.into(), "", 40.0).unwrap(),
    ];
    record_collection(pool, device.parse().unwrap(), at, &samples)
        .await
        .unwrap();
}

async fn add_gpu(pool: &PgPool, device: &str) {
    sqlx::query(
        "INSERT INTO metrics.gpus (device_id, index, name, vendor, has_vram) \
         VALUES ($1::text::uuid, 0, 'RTX 4090', 'nvidia', true)",
    )
    .bind(device)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn met_003_004_006_a_device_reads_its_readings_series_and_gpus_with_k1_names(pool: PgPool) {
    let now = database_now(&pool).await;
    record(&pool, PVE_01, now - TimeDelta::seconds(30), 55.0).await;
    add_gpu(&pool, PVE_01).await;
    let (schema, _) = schema_with_clock(&pool, Arc::new(FixedClock(now)));

    let result = data(&schema, DETAIL).await;

    let device = &result["device"];
    assert_eq!(device["readings"]["cpuPercent"], 55.0);
    assert_eq!(device["readings"]["memoryPercent"], 40.0);
    assert_eq!(device["readings"]["vramGpu"], Value::Null);
    let series = device["metrics"].as_array().unwrap();
    let kinds: Vec<&Value> = series.iter().map(|one| &one["kind"]).collect();
    assert_eq!(kinds, [&json!("CPU_PERCENT"), &json!("MEMORY_PERCENT")]);
    assert_eq!(series[0]["points"].as_array().unwrap().len(), 1);
    assert_eq!(series[0]["points"][0]["value"], 55.0);
    assert_eq!(
        device["gpus"],
        json!([{ "index": 0, "name": "RTX 4090", "vendor": "NVIDIA", "hasVram": true }])
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn met_004_a_device_without_a_recent_sample_has_null_readings_and_empty_lists(pool: PgPool) {
    let now = database_now(&pool).await;
    record(&pool, PVE_01, now - TimeDelta::hours(2), 55.0).await;
    let (schema, _) = schema_with_clock(&pool, Arc::new(FixedClock(now)));

    let result = data(&schema, DETAIL).await;

    assert_eq!(result["device"]["readings"], Value::Null);
    assert_eq!(result["device"]["gpus"], json!([]));
    assert_eq!(result["device"]["metrics"], json!([]));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn api_005_readings_and_gpus_cost_a_constant_few_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let now = database_now(&pool).await;
    record(&pool, PVE_01, now - TimeDelta::seconds(30), 55.0).await;
    add_gpu(&pool, PVE_01).await;
    let (counted, statements) = counting_pool(&pool).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(clock.clone())
        .data(readings_loader(counted.clone(), clock).delay(BATCH_WINDOW))
        .data(gpus_loader(counted.clone()).delay(BATCH_WINDOW))
        .finish();

    let few = data(&schema, LIST).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);
    add_bulk(&pool, SITES, 150).await;
    let many = data(&schema, LIST).await;
    let many_statements = statements.load(Ordering::SeqCst);

    for (result, count) in [(&few, 13), (&many, 163)] {
        assert_eq!(names(&result["devices"]).len(), count);
        let pve = result["devices"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| edge["node"]["name"] == "pve-01")
            .unwrap();
        assert_eq!(pve["node"]["readings"]["cpuPercent"], 55.0);
        assert_eq!(pve["node"]["gpus"][0]["name"], "RTX 4090");
    }
    // The page, its count, one batch of readings, one batch of GPUs. A per-device read costs
    // one statement per device and field.
    assert!(
        few_statements <= 12 && many_statements <= 12,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}
