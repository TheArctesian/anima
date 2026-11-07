//! Rounds against a test SSH device on loopback: one link per collection with the exact
//! commands and nothing else, the samples stored and published, rates on the second round,
//! one event per change, and Windows skipped without a link (MET-001, MET-002, MET-005,
//! CHK-012, P2-003, D-4, D-10, D-11).

use std::{
    io::Write as _,
    sync::{Arc, Mutex},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::sync::broadcast::{Receiver, error::TryRecvError};

use super::server::{PASSWORD, Server, host_key, serve};
use crate::{
    clock::Clock,
    credentials::{self, Secret, SshSecret, Vault},
    events::Bus,
    ids::DeviceId,
    metrics::{
        Feed, MetricKind, MetricsAdded,
        collector::{Collecting, Context, round::Collector},
        tests::captures::{
            DF_LINUX, PROC_LOADAVG, PROC_MEMINFO, PROC_NET_DEV, PROC_STAT, PROC_UPTIME,
        },
    },
    targets::AddressPolicy,
};

const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// Linux, no host key accepted: every link fails.
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const WIN_DESKTOP: &str = "0190a000-0000-7000-8002-00000000000d";
const OPERATOR: &str = "0190a000-0000-7000-8000-000000000001";

/// The command lines of a Linux collection, in order (D-3).
const LINUX_LINES: [&str; 6] = [
    "cat /proc/stat",
    "cat /proc/meminfo",
    "cat /proc/net/dev",
    "cat /proc/uptime",
    "cat /proc/loadavg",
    "df -P -B1",
];

/// A clock the test moves.
struct Steps(Mutex<DateTime<Utc>>);

impl Steps {
    fn advance(&self, seconds: i64) {
        *self.0.lock().unwrap() += TimeDelta::seconds(seconds);
    }
}

impl Clock for Steps {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

/// `docker-host` on a test server with its password stored; `pve-01` and `win-desktop` also
/// set to SSH metrics. The host key is accepted when `accepted`.
struct Bench {
    server: Server,
    clock: Arc<Steps>,
    feed: Receiver<MetricsAdded>,
    collector: Collector,
}

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

async fn execute(pool: &PgPool, sql: &str, id: &str) {
    sqlx::query(sql).bind(id).execute(pool).await.unwrap();
}

async fn bench(pool: &PgPool, accepted: bool) -> Bench {
    let server = serve().await;
    for (line, stdout) in LINUX_LINES.iter().zip([
        PROC_STAT,
        PROC_MEMINFO,
        PROC_NET_DEV,
        PROC_UPTIME,
        PROC_LOADAVG,
        DF_LINUX,
    ]) {
        server.answer(line, stdout);
    }
    let vault = vault();
    prepare(pool, &vault, server.port, accepted).await;
    let start: DateTime<Utc> = sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    let clock = Arc::new(Steps(Mutex::new(start - TimeDelta::minutes(10))));
    let feed = Feed::new();
    let subscribed = feed.subscribe();
    let collector = Collector::new(Context {
        db: pool.clone(),
        feed,
        bus: Bus::new(64),
        clock: Arc::clone(&clock) as Arc<dyn Clock>,
        collecting: Collecting {
            vault,
            policy: AddressPolicy::default().allowing_loopback(),
        },
    });
    Bench {
        server,
        clock,
        feed: subscribed,
        collector,
    }
}

/// The operator, the devices set to SSH metrics, `docker-host` pointed at the server on
/// `port` with its password, and its host key when `accepted`.
async fn prepare(pool: &PgPool, vault: &Vault, port: u16, accepted: bool) {
    execute(
        pool,
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::text::uuid, 'operator', '$argon2id$x', now())",
        OPERATOR,
    )
    .await;
    if accepted {
        accept(pool).await;
    }
    for id in [DOCKER_HOST, PVE_01, WIN_DESKTOP] {
        set_source(pool, id, "ssh").await;
    }
    execute(
        pool,
        "update devices.devices set host = '127.0.0.1' where id = $1::text::uuid",
        DOCKER_HOST,
    )
    .await;
    sqlx::query("update devices.ssh_settings set port = $2 where device_id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(i32::from(port))
        .execute(pool)
        .await
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let password = SshSecret::Password(Secret::from(PASSWORD.to_owned()));
    credentials::store_ssh(&mut conn, vault, device(DOCKER_HOST), password)
        .await
        .unwrap();
}

/// A vault over a throwaway key in a temporary file (SEC-004).
fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([5; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// Accepts the test server's host key for `docker-host`.
async fn accept(pool: &PgPool) {
    let key = host_key();
    sqlx::query(
        "insert into sessions.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1::text::uuid, $2, $3, $4, now(), now(), $5::text::uuid)",
    )
    .bind(DOCKER_HOST)
    .bind(key.algorithm())
    .bind(key.fingerprint().as_str().to_owned())
    .bind(key.to_base64())
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
}

async fn set_source(pool: &PgPool, id: &str, source: &str) {
    sqlx::query("update devices.devices set metrics_source = $2 where id = $1::text::uuid")
        .bind(id)
        .bind(source)
        .execute(pool)
        .await
        .unwrap();
}

/// The events stored, oldest first: severity, device name and message.
async fn events(pool: &PgPool) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "select severity, device_name, message from events.event_log \
         where source = 'device' order by at, device_name",
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
    assert_eq!(bench.server.heard(), LINUX_LINES);
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
    assert_eq!(bench.server.heard().len(), 12);
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
    assert!(events(&pool).await.is_empty(), "a partial collection works");
}
