//! Rounds against a test SSH device on loopback: one link per collection with the exact
//! commands and nothing else, the samples stored and published, rates on the second round,
//! one event per change, and Windows skipped without a link (MET-001, MET-002, MET-005,
//! CHK-012, P2-003, D-4, D-10, D-11).

use sqlx::PgPool;
use tokio::sync::broadcast::error::TryRecvError;

use super::{
    bench::{DOCKER_HOST, LINUX_GPU_LINES, LINUX_LINES, accept, bench, events, set_source},
    server::Server,
};
use crate::{
    clock::Clock as _,
    metrics::{MetricKind, MetricsAdded},
};

fn value(added: &MetricsAdded, kind: MetricKind, label: &str) -> Option<f64> {
    added
        .samples
        .iter()
        .find(|sample| sample.kind == kind && sample.label == label)
        .map(|sample| sample.value)
}

/// The Linux answers 30 seconds on: 600 more ticks, 150 idle; eth0 3 MB in, 600 kB out more.
fn later_counters(server: &Server) {
    server.answer("cat /proc/stat", b"cpu  5155 356 584 3849 23 23 0 0 0 0\n");
    server.answer(
        "cat /proc/net/dev",
        b"Inter-|   Receive\n face |bytes\n  \
eth0: 990654321 654321    0    0    0     0          0       100 124056789  234567    0    0    0     0       0          0\n",
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_001_a_round_runs_the_exact_commands_over_one_link_stores_and_publishes(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench.collector.round().await;
    assert_eq!(bench.server.connections(), 1, "one link, Windows skipped");
    assert_eq!(
        bench.server.heard(),
        [&LINUX_LINES[..], &LINUX_GPU_LINES[..]].concat(),
        "the first collection also looks for GPUs (MET-006)"
    );
    let first = bench.feed.try_recv().unwrap();
    assert_eq!(first.device_id.as_str(), DOCKER_HOST);
    assert_eq!(first.at, bench.clock.now());
    assert_eq!(
        value(&first, MetricKind::CpuPercent, ""),
        None,
        "no rate yet"
    );
    assert_eq!(value(&first, MetricKind::MemoryPercent, ""), Some(50.0));
    assert_eq!(value(&first, MetricKind::DiskPercent, "/"), Some(25.0));
    assert_eq!(first.readings.memory_percent, Some(50.0));
    assert_eq!(first.readings.disk_percent, Some(50.0), "the fullest mount");
    assert_eq!(bench.feed.try_recv(), Err(TryRecvError::Empty));
    let stored: i64 = sqlx::query_scalar("select count(*) from metrics.samples")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(usize::try_from(stored).unwrap(), first.samples.len());

    later_counters(&bench.server);
    bench.clock.advance(30);
    bench.collector.round().await;
    assert_eq!(
        bench.server.connections(),
        2,
        "a new link per collection (D-11)"
    );
    assert_eq!(
        bench.server.heard().len(),
        15,
        "no GPU found, so no GPU read the second time"
    );
    let second = bench.feed.try_recv().unwrap();
    assert_eq!(value(&second, MetricKind::CpuPercent, ""), Some(75.0));
    assert_eq!(
        value(&second, MetricKind::NetworkInBps, "eth0"),
        Some(100_000.0)
    );
    assert_eq!(
        value(&second, MetricKind::NetworkOutBps, "eth0"),
        Some(20_000.0)
    );
    assert_eq!(second.readings.cpu_percent, Some(75.0));
    assert!(
        bench
            .server
            .heard()
            .iter()
            .all(|line| !line.starts_with('!'))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d10_one_warning_when_failing_starts_and_one_info_when_it_works_again(pool: PgPool) {
    let mut bench = bench(&pool, false).await;
    bench.collector.round().await;
    bench.clock.advance(30);
    bench.collector.round().await;
    assert_eq!(
        bench.server.connections(),
        0,
        "no accepted key, no connection"
    );
    let warning =
        "Metrics cannot be collected from the device; the hub tries again every 30 seconds";
    let failing = |name: &str| ("warning".to_owned(), name.to_owned(), warning.to_owned());
    assert_eq!(
        events(&pool).await,
        [failing("docker-host"), failing("pve-01")],
        "one each, not one per round; none for Windows"
    );
    accept(&pool).await;
    bench.clock.advance(30);
    bench.collector.round().await;
    bench.clock.advance(30);
    bench.collector.round().await;
    let stored = events(&pool).await;
    assert_eq!(stored.len(), 3, "{stored:?}");
    assert_eq!(
        stored[2],
        (
            "info".to_owned(),
            "docker-host".to_owned(),
            "Metrics are collected from the device again".to_owned()
        )
    );
    for (_, _, message) in &stored {
        assert!(!message.contains("127.0.0.1"), "{message}");
        assert!(
            !message.contains(&bench.server.port.to_string()),
            "{message}"
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d4_a_device_that_leaves_the_list_loses_its_previous_reading(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench.collector.round().await;
    let _first = bench.feed.try_recv().unwrap();
    set_source(&pool, DOCKER_HOST, "none").await;
    bench.clock.advance(30);
    bench.collector.round().await;
    assert_eq!(bench.server.connections(), 1, "not collected while off");
    set_source(&pool, DOCKER_HOST, "ssh").await;
    later_counters(&bench.server);
    bench.clock.advance(30);
    bench.collector.round().await;
    let back = bench.feed.try_recv().unwrap();
    assert_eq!(
        value(&back, MetricKind::CpuPercent, ""),
        None,
        "rates start again"
    );
    assert!(value(&back, MetricKind::MemoryPercent, "").is_some());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_002_a_failed_command_leaves_its_source_out_and_the_rest_is_stored(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench
        .server
        .answers
        .lock()
        .unwrap()
        .remove("cat /proc/meminfo");
    bench.collector.round().await;
    let added = bench.feed.try_recv().unwrap();
    assert_eq!(value(&added, MetricKind::MemoryPercent, ""), None);
    assert_eq!(value(&added, MetricKind::DiskPercent, "/"), Some(25.0));
    assert_eq!(value(&added, MetricKind::Load1, ""), Some(0.20));
    let stored = events(&pool).await;
    assert!(
        stored.iter().all(|(_, name, _)| name != "docker-host"),
        "a partial collection works: {stored:?}"
    );
}
