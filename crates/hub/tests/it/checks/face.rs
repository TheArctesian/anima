//! The checks face against the database: heartbeats and aggregates, uptime, status,
//! writes, sealed secrets and retention (CHK-002–006, CHK-011, DB-008, DB-009).
//!
//! Beat times come from the database clock, because the partition functions accept only
//! instants near it.

pub(super) mod batching;
mod beats;
mod configs;
mod due;
mod http_urls;
mod retention;
mod secrets;
mod snmp;
mod statuses;
mod uptime;
mod writes;

use std::io::Write as _;

use anima_hub::{
    checks::{
        self, Beat, CheckSettings, CheckState, Definition, NewCheck, Recorded,
        config::{CheckConfig, IpVersion},
    },
    credentials::Vault,
    ids::{CheckId, DeviceId},
    status::Status,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use sqlx::PgPool;

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";
const TRUENAS: &str = "0190a000-0000-7000-8002-000000000007";
/// The "Ping" and "SSH port" checks of `pve-01` in the fixtures.
const PVE_01_PING: &str = "0190a000-0000-7000-8003-000000000001";
const PVE_01_SSH: &str = "0190a000-0000-7000-8003-000000000002";

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

fn check(text: &str) -> CheckId {
    text.parse().unwrap()
}

fn vault_with(byte: u8) -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([byte; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

fn vault() -> Vault {
    vault_with(7)
}

/// The database clock, truncated to the second so stored and expected times compare equal.
async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(TimeDelta::seconds(1)).unwrap()
}

fn icmp() -> Definition {
    Definition::Config(CheckConfig::Icmp {
        timeout_ms: 2000,
        ip_version: IpVersion::V4,
    })
}

fn new_check(device_text: &str, name: &str, definition: Definition) -> NewCheck {
    NewCheck {
        device: device(device_text),
        parent: None,
        name: name.to_owned(),
        definition,
        settings: CheckSettings::default(),
    }
}

/// Adds a check and returns its id.
async fn create(pool: &PgPool, new: NewCheck) -> CheckId {
    let mut conn = pool.acquire().await.unwrap();
    checks::create_check(&mut conn, &vault(), new)
        .await
        .unwrap()
        .id
}

/// A beat at `at` with the given heartbeat status, leaving the check in that status.
fn beat(at: DateTime<Utc>, status: Status, response_time_ms: Option<u32>) -> Beat {
    Beat {
        at,
        status,
        response_time_ms,
        message: None,
        state: CheckState {
            status,
            failures: 0,
            next_due_at: Some(at + TimeDelta::seconds(60)),
        },
        resolved_ip: None,
        cert_not_after: None,
    }
}

async fn record(pool: &PgPool, id: CheckId, beat: &Beat) {
    let mut conn = pool.acquire().await.unwrap();
    let recorded = checks::record_beat(&mut conn, id, beat).await.unwrap();
    assert_eq!(recorded, Recorded::Written);
}

/// Sets the stored status of a check directly.
async fn set_status(pool: &PgPool, id: &str, status: &str) {
    sqlx::query("UPDATE checks.checks SET status = $2 WHERE id = $1::text::uuid")
        .bind(id)
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
}

async fn count(pool: &PgPool, sql: &str, id: CheckId) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}
