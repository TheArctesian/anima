//! GPU discovery and GPU samples in rounds against a test SSH device on loopback: found at the
//! first collection and stored, only the known vendors read after, found again after a
//! restart, a hung GPU tool and the GPU window that end only the GPU reads, numbers kept
//! across a failed collection, and a deny-listed device that gets nothing (MET-006, MET-007,
//! MET-009, D-11, P2-004).

use std::time::Duration;

use agent_core::readings::captures::NVIDIA_SMI_TWO;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    bench::{DOCKER_HOST, LINUX_GPU_LINES, LINUX_LINES, accept, bench, bench_with, device},
    gpus::amd_card,
};
use crate::{
    clock::Clock as _,
    metrics::{self, Gpu, GpuVendor, MetricKind, MetricsAdded},
    targets::AddressPolicy,
};

/// `docker-host`'s stored GPUs.
async fn stored(pool: &PgPool) -> Vec<Gpu> {
    let id = device(DOCKER_HOST);
    metrics::gpus(pool, &[id])
        .await
        .unwrap()
        .remove(&id)
        .unwrap_or_default()
}

/// The stored discovery of `docker-host`: when, and the boot time seen then.
async fn discovery(pool: &PgPool) -> Option<(DateTime<Utc>, Option<DateTime<Utc>>)> {
    sqlx::query_as("select found_at, boot_at from metrics.discovery where device_id = $1")
        .bind(device(DOCKER_HOST))
        .fetch_optional(pool)
        .await
        .unwrap()
}

/// The GPU series stored: kind, label and GPU index.
async fn gpu_series(pool: &PgPool) -> Vec<(String, String, Option<i32>)> {
    sqlx::query_as(
        "select kind, label, gpu_index from metrics.series \
         where kind like 'gpu%' or kind like 'vram%' order by label, kind",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

fn value(added: &MetricsAdded, kind: MetricKind, label: &str) -> Option<f64> {
    added
        .samples
        .iter()
        .find(|sample| sample.kind == kind && sample.label == label)
        .map(|sample| sample.value)
}

/// The metrics events stored for `docker-host`.
async fn collection_events(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "select count(*) from events.event_log \
         where source = 'metrics' and device_name = 'docker-host'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// The lines heard since `from`.
fn heard_since(heard: &[String], from: usize) -> Vec<&str> {
    heard[from..].iter().map(String::as_str).collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_006_the_first_collection_finds_and_stores_the_gpus_with_their_samples(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench.server.answer(LINUX_GPU_LINES[0], NVIDIA_SMI_TWO);
    bench.server.answer(LINUX_GPU_LINES[1], &amd_card(1));
    bench.collector.round().await;

    let gpu = |index, name: &str, vendor| Gpu {
        index,
        name: name.to_owned(),
        vendor,
        has_vram: true,
    };
    assert_eq!(
        stored(&pool).await,
        [
            gpu(0, "NVIDIA GeForce RTX 4090", GpuVendor::Nvidia),
            gpu(1, "Tesla T4, Quadro, Mixed", GpuVendor::Nvidia),
            gpu(2, "AMD GPU", GpuVendor::Amd),
        ],
        "Intel's tool is missing: no Intel GPU"
    );
    let now = bench.clock.now();
    let (found_at, boot_at) = discovery(&pool).await.unwrap();
    assert_eq!(found_at, now);
    let uptime = chrono::TimeDelta::milliseconds(350_735_470);
    let off = (boot_at.unwrap() - (now - uptime)).abs();
    assert!(
        off < chrono::TimeDelta::milliseconds(1),
        "from /proc/uptime: {off}"
    );

    let added = bench.feed.try_recv().unwrap();
    assert_eq!(value(&added, MetricKind::GpuPercent, "0"), Some(97.0));
    assert_eq!(value(&added, MetricKind::GpuPercent, "2"), Some(37.0));
    assert!(value(&added, MetricKind::MemoryPercent, "").is_some());
    let series = gpu_series(&pool).await;
    assert_eq!(series.len(), 13, "{series:?}");
    assert!(
        series
            .iter()
            .all(|(_, label, index)| index.map(|index| index.to_string()) == Some(label.clone())),
        "{series:?}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_006_later_rounds_read_only_known_vendors_and_a_restart_looks_again(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench.server.answer(LINUX_GPU_LINES[0], NVIDIA_SMI_TWO);
    bench.collector.round().await;
    let first = bench.server.heard().len();

    bench.clock.advance(30);
    bench.collector.round().await;
    let heard = bench.server.heard();
    let mut expected = LINUX_LINES.to_vec();
    expected.push(LINUX_GPU_LINES[0]);
    assert_eq!(heard_since(&heard, first), expected, "only nvidia-smi");

    // The device restarted: its uptime is 100 seconds, so it booted 4 days later than before.
    bench.server.answer("cat /proc/uptime", b"100.00 50.00\n");
    bench.server.answer(LINUX_GPU_LINES[1], &amd_card(0));
    bench.clock.advance(30);
    let second = bench.server.heard().len();
    bench.collector.round().await;
    let heard = bench.server.heard();
    assert_eq!(
        heard_since(&heard, second),
        [&LINUX_LINES[..], &LINUX_GPU_LINES[..]].concat(),
        "every read again"
    );
    let vendors: Vec<GpuVendor> = stored(&pool).await.iter().map(|gpu| gpu.vendor).collect();
    assert_eq!(
        vendors,
        [GpuVendor::Nvidia, GpuVendor::Nvidia, GpuVendor::Amd]
    );
    let (found_at, boot_at) = discovery(&pool).await.unwrap();
    let now = bench.clock.now();
    assert_eq!(found_at, now);
    assert_eq!(boot_at, Some(now - chrono::TimeDelta::seconds(100)));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_006_a_hung_gpu_tool_ends_only_its_read_and_the_next_round_looks_again(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench.server.stall(LINUX_GPU_LINES[0]);
    bench.server.answer(LINUX_GPU_LINES[1], &amd_card(1));
    bench.collector.round().await;
    let added = bench.feed.try_recv().unwrap();
    assert!(value(&added, MetricKind::MemoryPercent, "").is_some());
    assert_eq!(
        bench.server.heard(),
        [&LINUX_LINES[..], &LINUX_GPU_LINES[..]].concat(),
        "the reads after the hung one still run"
    );
    assert_eq!(
        stored(&pool).await.len(),
        1,
        "AMD answered, so its card is stored"
    );
    assert_eq!(value(&added, MetricKind::GpuPercent, "0"), Some(37.0));
    assert_eq!(collection_events(&pool).await, 0, "the collection worked");

    bench.server.answer(LINUX_GPU_LINES[0], NVIDIA_SMI_TWO);
    bench.server.unstall(LINUX_GPU_LINES[0]);
    bench.clock.advance(30);
    let first = bench.server.heard().len();
    bench.collector.round().await;
    let heard = bench.server.heard();
    let mut expected = LINUX_LINES.to_vec();
    expected.extend(&LINUX_GPU_LINES[..2]);
    assert_eq!(
        heard_since(&heard, first),
        expected,
        "the known AMD and the pending NVIDIA"
    );
    let numbers: Vec<(i32, GpuVendor)> = stored(&pool)
        .await
        .iter()
        .map(|gpu| (gpu.index, gpu.vendor))
        .collect();
    assert_eq!(
        numbers,
        [
            (0, GpuVendor::Amd),
            (1, GpuVendor::Nvidia),
            (2, GpuVendor::Nvidia)
        ],
        "the AMD card keeps its number (MET-009)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d11_gpu_reads_stop_at_the_gpu_window_and_the_collection_is_stored(pool: PgPool) {
    let policy = AddressPolicy::default().allowing_loopback();
    let mut bench = bench_with(&pool, true, policy, Duration::from_secs(2)).await;
    bench.server.stall(LINUX_GPU_LINES[0]);
    bench.server.answer(LINUX_GPU_LINES[1], &amd_card(1));
    let started = std::time::Instant::now();
    bench.collector.round().await;
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    let mut expected = LINUX_LINES.to_vec();
    expected.push(LINUX_GPU_LINES[0]);
    assert_eq!(
        bench.server.heard(),
        expected,
        "no GPU read after the window"
    );
    let added = bench.feed.try_recv().unwrap();
    assert!(value(&added, MetricKind::MemoryPercent, "").is_some());
    assert_eq!(stored(&pool).await, []);
    assert_eq!(collection_events(&pool).await, 0, "the collection worked");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_009_after_a_failed_collection_the_next_looks_again_and_keeps_the_numbers(
    pool: PgPool,
) {
    let mut bench = bench(&pool, true).await;
    bench.server.answer(LINUX_GPU_LINES[0], NVIDIA_SMI_TWO);
    bench.server.answer(LINUX_GPU_LINES[1], &amd_card(1));
    bench.collector.round().await;
    sqlx::query("delete from trust.host_keys")
        .execute(&pool)
        .await
        .unwrap();
    bench.clock.advance(30);
    bench.collector.round().await;
    let failed = bench.server.heard().len();
    // The device rebooted unseen and its first NVIDIA GPU is gone.
    let second_line = NVIDIA_SMI_TWO
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap()
        + 1;
    bench
        .server
        .answer(LINUX_GPU_LINES[0], &NVIDIA_SMI_TWO[second_line..]);
    accept(&pool).await;
    bench.clock.advance(30);
    bench.collector.round().await;
    let heard = bench.server.heard();
    assert_eq!(
        heard_since(&heard, failed),
        [&LINUX_LINES[..], &LINUX_GPU_LINES[..]].concat(),
        "the device may have restarted unseen"
    );
    let numbers: Vec<(i32, GpuVendor)> = stored(&pool)
        .await
        .iter()
        .map(|gpu| (gpu.index, gpu.vendor))
        .collect();
    assert_eq!(
        numbers,
        [(1, GpuVendor::Nvidia), (2, GpuVendor::Amd)],
        "the GPUs left keep their numbers (MET-009)"
    );
    let _first = bench.feed.try_recv().unwrap();
    let added = bench.feed.try_recv().unwrap();
    assert_eq!(value(&added, MetricKind::GpuPercent, "1"), Some(3.0));
    assert_eq!(value(&added, MetricKind::GpuPercent, "2"), Some(37.0));
    assert_eq!(value(&added, MetricKind::GpuPercent, "0"), None);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_a_deny_listed_device_gets_no_gpu_discovery_and_no_samples(pool: PgPool) {
    let policy = AddressPolicy::new(vec!["127.0.0.0/8".parse().unwrap()]).allowing_loopback();
    let mut bench = bench_with(&pool, true, policy, Duration::from_secs(18)).await;
    bench.server.answer(LINUX_GPU_LINES[0], NVIDIA_SMI_TWO);
    bench.collector.round().await;
    bench.clock.advance(30);
    bench.collector.round().await;
    assert_eq!(
        bench.server.connections(),
        0,
        "the deny list wins over loopback"
    );
    assert_eq!(bench.server.heard(), Vec::<String>::new());
    assert_eq!(discovery(&pool).await, None);
    assert_eq!(stored(&pool).await, []);
    let samples: i64 = sqlx::query_scalar("select count(*) from metrics.series")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(samples, 0, "no series, so no samples");
    assert!(bench.feed.try_recv().is_err());
}
