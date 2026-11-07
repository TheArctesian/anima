//! The world a round test starts from: `docker-host` on a test SSH server with its key
//! accepted and its password stored, `pve-01` and `win-desktop` also set to SSH metrics, a
//! clock the test moves, and the stored events.

use std::{
    io::Write as _,
    sync::{Arc, Mutex},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::sync::broadcast::Receiver;

use super::server::{PASSWORD, Server, host_key, serve};
use crate::{
    clock::Clock,
    credentials::{self, Secret, SshSecret, Vault},
    events::Bus,
    ids::DeviceId,
    metrics::{
        Feed, MetricsAdded,
        collector::{Collecting, Context, round::Collector},
        tests::captures::{
            DF_LINUX, PROC_LOADAVG, PROC_MEMINFO, PROC_NET_DEV, PROC_STAT, PROC_UPTIME,
        },
    },
    targets::AddressPolicy,
};

pub(super) const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// Linux, no host key accepted: every link fails.
pub(super) const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
pub(super) const WIN_DESKTOP: &str = "0190a000-0000-7000-8002-00000000000d";
pub(super) const OPERATOR: &str = "0190a000-0000-7000-8000-000000000001";

/// The command lines of a Linux collection, in order (D-3).
pub(super) const LINUX_LINES: [&str; 6] = [
    "cat /proc/stat",
    "cat /proc/meminfo",
    "cat /proc/net/dev",
    "cat /proc/uptime",
    "cat /proc/loadavg",
    "df -P -B1",
];

/// A clock the test moves.
pub(super) struct Steps(Mutex<DateTime<Utc>>);

impl Steps {
    pub(super) fn advance(&self, seconds: i64) {
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
pub(super) struct Bench {
    pub(super) server: Server,
    pub(super) clock: Arc<Steps>,
    pub(super) feed: Receiver<MetricsAdded>,
    pub(super) collector: Collector,
}

pub(super) fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

pub(super) async fn execute(pool: &PgPool, sql: &str, id: &str) {
    sqlx::query(sql).bind(id).execute(pool).await.unwrap();
}

pub(super) async fn bench(pool: &PgPool, accepted: bool) -> Bench {
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
pub(super) async fn prepare(pool: &PgPool, vault: &Vault, port: u16, accepted: bool) {
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
pub(super) fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([5; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// Accepts the test server's host key for `docker-host`.
pub(super) async fn accept(pool: &PgPool) {
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

pub(super) async fn set_source(pool: &PgPool, id: &str, source: &str) {
    sqlx::query("update devices.devices set metrics_source = $2 where id = $1::text::uuid")
        .bind(id)
        .bind(source)
        .execute(pool)
        .await
        .unwrap();
}

/// The events stored, oldest first: severity, device name and message.
pub(super) async fn events(pool: &PgPool) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "select severity, device_name, message from events.event_log \
         where source = 'metrics' order by at, device_name",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}
