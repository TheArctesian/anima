//! Devices, checks, listeners and readers that the tick tests share.

use std::{
    io::Write as _,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use rand::{SeedableRng as _, rngs::StdRng};
use sqlx::PgPool;

use crate::{
    checks::{
        self, Check, CheckSettings, Definition, NewCheck,
        config::CheckConfig,
        scheduler::{
            Probing,
            attempt::Context,
            tick::{Limits, Scheduler},
        },
    },
    credentials::Vault,
    events::Bus,
    ids::{CheckId, DeviceId},
    targets::AddressPolicy,
};

/// `docker-host` in the K6 fixtures: no maintenance.
pub const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// `jellyfin` in the K6 fixtures: the manual maintenance flag is set.
pub const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";

pub fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

/// A vault whose key is 32 bytes of `byte`.
pub fn vault_with(byte: u8) -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([byte; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// The vault the test schedulers use.
pub fn vault() -> Vault {
    vault_with(7)
}

pub fn rng() -> StdRng {
    StdRng::seed_from_u64(7)
}

/// What the attempts of a test share, with this policy.
fn context_with(pool: &PgPool, bus: &Bus, policy: AddressPolicy) -> Context {
    Context::new(
        pool.clone(),
        bus.clone(),
        Probing {
            vault: vault(),
            policy,
            localapi: None,
        },
    )
}

/// What one attempt needs to reach the local listeners.
pub fn context(pool: &PgPool) -> Context {
    context_with(
        pool,
        &Bus::new(16),
        AddressPolicy::default().allowing_loopback(),
    )
}

/// A scheduler with this policy.
pub fn scheduler_with(pool: &PgPool, bus: &Bus, policy: AddressPolicy) -> Scheduler {
    let ctx = context_with(pool, bus, policy);
    Scheduler::new(
        ctx,
        Limits {
            probes: 32,
            due_per_tick: 256,
        },
    )
}

/// A scheduler whose probes may reach the local listeners.
pub fn scheduler(pool: &PgPool, bus: &Bus) -> Scheduler {
    scheduler_with(pool, bus, AddressPolicy::default().allowing_loopback())
}

/// A scheduler that reaches the local listeners within these limits.
pub fn limited(pool: &PgPool, limits: Limits) -> Scheduler {
    let ctx = Context::new(
        pool.clone(),
        Bus::new(16),
        Probing {
            vault: vault(),
            policy: AddressPolicy::default().allowing_loopback(),
            localapi: None,
        },
    );
    Scheduler::new(ctx, limits)
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

/// A check named `name` with [`settings`] and no group.
pub fn new_check(device: DeviceId, name: &str, definition: Definition, retries: u8) -> NewCheck {
    NewCheck {
        device,
        parent: None,
        name: name.to_owned(),
        definition,
        settings: settings(retries),
    }
}

/// Adds a check named "Service port"; its first due time is unset.
pub async fn add(pool: &PgPool, device: DeviceId, definition: Definition, retries: u8) -> CheckId {
    add_check(pool, new_check(device, "Service port", definition, retries)).await
}

pub async fn add_check(pool: &PgPool, new: NewCheck) -> CheckId {
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

    /// Waits until `count` connections were accepted; panics after 5 seconds.
    pub async fn wait_for(&self, count: usize) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while self.accepted() < count {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{} accepted",
                self.accepted()
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// A TLS check: the handshake never completes, so the attempt stays in flight for 2 s.
    pub fn silent_tls(&self) -> Definition {
        Definition::Config(CheckConfig::Tls {
            port: self.port,
            server_name: None,
            timeout_ms: 2000,
        })
    }
}

/// Gives an attempt that should not start the time to reach the listener.
pub async fn settle_briefly() {
    tokio::time::sleep(Duration::from_millis(300)).await;
}

/// Makes every event write fail until [`allow_event_writes`].
pub async fn fail_event_writes(pool: &PgPool) {
    for sql in [
        "CREATE FUNCTION events.refuse() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'refused'; END $$",
        "CREATE TRIGGER refuse BEFORE INSERT ON events.event_log \
         FOR EACH ROW EXECUTE FUNCTION events.refuse()",
    ] {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

pub async fn allow_event_writes(pool: &PgPool) {
    sqlx::query("DROP TRIGGER refuse ON events.event_log")
        .execute(pool)
        .await
        .unwrap();
}
