//! Devices, checks, listeners and readers that the tick tests share.

use std::{
    io::Write as _,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use rand::{SeedableRng as _, rngs::StdRng};
use sqlx::PgPool;

use crate::{
    checks::{
        self, Check, CheckSettings, Definition, NewCheck,
        config::CheckConfig,
        probe::AddressPolicy,
        scheduler::{Probing, attempt::Context, tick::Scheduler},
    },
    credentials::Vault,
    events::Bus,
    ids::{CheckId, DeviceId},
};

/// `docker-host` in the K6 fixtures: no maintenance.
pub const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// `jellyfin` in the K6 fixtures: the manual maintenance flag is set.
pub const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";

pub fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

pub fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7u8; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

pub fn rng() -> StdRng {
    StdRng::seed_from_u64(7)
}

/// A scheduler with this policy.
pub fn scheduler_with(pool: &PgPool, bus: &Bus, policy: AddressPolicy) -> Scheduler {
    let probing = Probing {
        vault: vault(),
        policy,
    };
    let ctx = Context {
        db: pool.clone(),
        bus: bus.clone(),
        probing,
    };
    Scheduler::new(ctx, 32)
}

/// A scheduler whose probes may reach the local listeners.
pub fn scheduler(pool: &PgPool, bus: &Bus) -> Scheduler {
    scheduler_with(pool, bus, AddressPolicy::default().allowing_loopback())
}

/// The database clock, truncated to the second: the partition functions accept only
/// instants near it.
pub async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(TimeDelta::seconds(1)).unwrap()
}

/// Points the device at 127.0.0.1.
pub async fn on_loopback(pool: &PgPool, text: &str) -> DeviceId {
    sqlx::query("UPDATE devices.devices SET host = '127.0.0.1' WHERE id = $1")
        .bind(device(text))
        .execute(pool)
        .await
        .unwrap();
    device(text)
}

/// Settings with a 10-second interval, a 5-second retry interval and these retries.
pub fn settings(retries: u8) -> CheckSettings {
    CheckSettings {
        interval_seconds: 10,
        retries,
        retry_interval_seconds: 5,
        ..CheckSettings::default()
    }
}

pub fn tcp(port: u16) -> Definition {
    Definition::Config(CheckConfig::Tcp {
        port,
        timeout_ms: 1000,
    })
}

/// Adds a check; its first due time is unset.
pub async fn add(pool: &PgPool, device: DeviceId, definition: Definition, retries: u8) -> CheckId {
    let new = NewCheck {
        device,
        parent: None,
        name: "Service port".to_owned(),
        definition,
        settings: settings(retries),
    };
    let mut conn = pool.acquire().await.unwrap();
    checks::create_check(&mut conn, &vault(), new)
        .await
        .unwrap()
        .id
}

/// Makes the check due at `at`.
pub async fn due_at(pool: &PgPool, id: CheckId, at: DateTime<Utc>) {
    sqlx::query("UPDATE checks.checks SET next_due_at = $2 WHERE id = $1")
        .bind(id)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
}

pub async fn stored(pool: &PgPool, id: CheckId) -> Check {
    checks::get(pool, id).await.unwrap()
}

/// The statuses of the check's heartbeats, oldest first.
pub async fn beats(pool: &PgPool, id: CheckId) -> Vec<String> {
    sqlx::query_scalar("SELECT status FROM checks.heartbeats WHERE check_id = $1 ORDER BY at")
        .bind(id)
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The device's events as (severity, source, message), oldest first.
pub async fn events_of(pool: &PgPool, device: DeviceId) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "SELECT severity, source, message FROM events.event_log WHERE device_id = $1 \
         ORDER BY at, id",
    )
    .bind(device)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// A port nothing listens on.
pub fn closed_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// A listener on 127.0.0.1 that accepts, holds every connection open and says nothing.
pub struct Listener {
    pub port: u16,
    pub accepted: Arc<AtomicUsize>,
}

impl Listener {
    pub async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepted);
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((tcp, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                held.push(tcp);
            }
        });
        Self { port, accepted }
    }

    pub fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    /// A TLS check: the handshake never completes, so the attempt stays in flight for 1 s.
    pub fn silent_tls(&self) -> Definition {
        Definition::Config(CheckConfig::Tls {
            port: self.port,
            server_name: None,
            timeout_ms: 1000,
        })
    }
}

/// Lets the attempts in flight reach the listener.
pub async fn settle_briefly() {
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
}
