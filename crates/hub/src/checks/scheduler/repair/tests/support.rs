//! Devices, ICMP checks, listeners and readers that the repair tests share.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use rand::{SeedableRng as _, rngs::StdRng};
use serde_json::Value;
use sqlx::PgPool;

use crate::{
    checks::{
        self, Beat, Check, CheckChange, CheckSettings, CheckState, Definition, NewCheck, Recorded,
        config::{CheckConfig, IpVersion},
        probe::{Failure, ProbeResult},
        scheduler::{
            Probing,
            attempt::Context,
            tick::{Limits, Scheduler},
        },
    },
    credentials::Vault,
    events::Bus,
    ids::{CheckId, DeviceId},
    status::Status,
    targets::AddressPolicy,
};

/// `docker-host` in the K6 fixtures: SSH on, no maintenance.
pub const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// `home-assistant` in the K6 fixtures: SSH off.
pub const HOME_ASSISTANT: &str = "0190a000-0000-7000-8002-000000000004";
/// `jellyfin` in the K6 fixtures: the manual maintenance flag is set.
pub const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";

/// A vault whose key is 32 bytes of 7; ICMP checks hold no secret.
pub fn vault() -> Vault {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use std::io::Write as _;
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// What one repair needs to reach the local listeners.
pub fn context(pool: &PgPool, bus: &Bus) -> Context {
    Context::new(
        pool.clone(),
        bus.clone(),
        Probing {
            vault: vault(),
            policy: AddressPolicy::default().allowing_loopback(),
            localapi: None,
        },
    )
}

/// A scheduler whose probes may reach the local listeners.
pub fn scheduler(pool: &PgPool, bus: &Bus) -> Scheduler {
    let limits = Limits {
        probes: 32,
        due_per_tick: 256,
    };
    Scheduler::new(context(pool, bus), limits)
}

/// One tick at `at`, waiting for its attempts and their repairs.
pub async fn tick_at(scheduler: &mut Scheduler, at: DateTime<Utc>) {
    scheduler.tick(at, &mut StdRng::seed_from_u64(7)).await;
    scheduler.finish_running(at).await;
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

/// Points the device at 127.0.0.1 and its SSH setting at `ssh_port`. A fixture write: the
/// devices domain has no public SSH write outside GraphQL.
pub async fn on_loopback(pool: &PgPool, text: &str, ssh_port: u16) -> DeviceId {
    let device: DeviceId = text.parse().unwrap();
    sqlx::query("UPDATE devices.devices SET host = '127.0.0.1' WHERE id = $1")
        .bind(device)
        .execute(pool)
        .await
        .unwrap();
    set_ssh_port(pool, device, ssh_port).await;
    device
}

pub async fn set_ssh_port(pool: &PgPool, device: DeviceId, port: u16) {
    sqlx::query("UPDATE devices.ssh_settings SET port = $2 WHERE device_id = $1")
        .bind(device)
        .bind(i32::from(port))
        .execute(pool)
        .await
        .unwrap();
}

/// An ICMP check over IPv6 to 127.0.0.1, which has no IPv6 address: every attempt fails at
/// once, whatever the machine allows for ICMP sockets.
pub fn silent_ping() -> CheckConfig {
    CheckConfig::Icmp {
        timeout_ms: 1000,
        ip_version: IpVersion::V6,
    }
}

/// Adds an ICMP check named "Ping" with a 10-second interval and `retries`, due at `at`.
pub async fn add_ping(pool: &PgPool, device: DeviceId, retries: u8, at: DateTime<Utc>) -> CheckId {
    add_check(pool, device, ("Ping", silent_ping(), retries), at).await
}

/// Adds a check named `name` with `config`, a 10-second interval and `retries`, due at `at`.
pub async fn add_check(
    pool: &PgPool,
    device: DeviceId,
    (name, config, retries): (&str, CheckConfig, u8),
    at: DateTime<Utc>,
) -> CheckId {
    let new = NewCheck {
        device,
        parent: None,
        name: name.to_owned(),
        definition: Definition::Config(config),
        settings: CheckSettings {
            interval_seconds: 10,
            retries,
            retry_interval_seconds: 5,
            ..CheckSettings::default()
        },
    };
    let mut conn = pool.acquire().await.unwrap();
    let id = checks::create_check(&mut conn, &vault(), new)
        .await
        .unwrap()
        .id;
    make_due(pool, id, at).await;
    id
}

pub async fn make_due(pool: &PgPool, id: CheckId, at: DateTime<Utc>) {
    sqlx::query("UPDATE checks.checks SET next_due_at = $2 WHERE id = $1")
        .bind(id)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
}

/// Records a beat with `status` at `at` through the checks domain; a Down beat counts one
/// failure.
pub async fn beat(pool: &PgPool, id: CheckId, status: Status, at: DateTime<Utc>) {
    let state = CheckState {
        status,
        failures: u32::from(status == Status::Down),
        next_due_at: Some(at + TimeDelta::seconds(10)),
    };
    let probe = if status == Status::Down {
        ProbeResult::failed(&Failure::NoReply)
    } else {
        ProbeResult::up(std::time::Duration::from_millis(3))
    };
    let beat = Beat::of(probe, (state, status), at);
    let mut conn = pool.acquire().await.unwrap();
    let recorded = checks::record_beat(&mut conn, id, &beat).await.unwrap();
    assert_eq!(recorded, Recorded::Written);
}

/// An operator's edit through the checks domain, which marks the check as edited.
pub async fn edit(pool: &PgPool, id: CheckId) {
    let change = CheckChange {
        degraded_threshold_ms: Some(2000),
        ..CheckChange::default()
    };
    let mut conn = pool.acquire().await.unwrap();
    checks::update_check(&mut conn, &vault(), id, change)
        .await
        .unwrap();
}

pub async fn stored(pool: &PgPool, id: CheckId) -> Check {
    checks::get(pool, id).await.unwrap()
}

pub async fn tried_at(pool: &PgPool, id: CheckId) -> Option<DateTime<Utc>> {
    sqlx::query_scalar("SELECT repair_tried_at FROM checks.checks WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The statuses of the check's heartbeats, oldest first.
pub async fn beats(pool: &PgPool, id: CheckId) -> Vec<String> {
    sqlx::query_scalar("SELECT status FROM checks.heartbeats WHERE check_id = $1 ORDER BY at")
        .bind(id)
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The device's events that are not repair events, oldest first.
pub async fn other_events(pool: &PgPool, device: DeviceId) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "SELECT severity, source, message FROM events.event_log \
         WHERE device_id = $1 AND message NOT LIKE 'Ping never answered%' ORDER BY at, id",
    )
    .bind(device)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// The device's repair events as (severity, source, message), oldest first.
pub async fn repair_events(pool: &PgPool, device: DeviceId) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "SELECT severity, source, message FROM events.event_log \
         WHERE device_id = $1 AND message LIKE 'Ping never answered%' ORDER BY at, id",
    )
    .bind(device)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// A stored audit entry of the check, without its id and time.
#[derive(Debug, PartialEq, sqlx::FromRow)]
pub struct Audited {
    pub operator_id: uuid::Uuid,
    pub operator_name: String,
    pub action: String,
    pub target_type: String,
    pub client: String,
    pub change: Value,
}

pub async fn audits_of(pool: &PgPool, id: CheckId) -> Vec<Audited> {
    sqlx::query_as(
        "SELECT operator_id, operator_name, action, target_type, host(client_addr) AS client, \
                change \
         FROM events.audit_log WHERE target_id = $1 ORDER BY at, id",
    )
    .bind(id.to_string())
    .fetch_all(pool)
    .await
    .unwrap()
}

/// A port nothing listens on.
pub fn closed_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// A listener on 127.0.0.1 that accepts, holds every connection open and counts them.
pub struct Listener {
    pub port: u16,
    accepted: Arc<AtomicUsize>,
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
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while self.accepted() < count {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{} accepted",
                self.accepted()
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}

/// Gives a connect that should not happen the time to reach the listener.
pub async fn settle_briefly() {
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
}
