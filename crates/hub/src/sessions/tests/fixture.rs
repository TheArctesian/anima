//! The world both tunnels' wire tests start from, and what a session leaves behind: the
//! operator, `docker-host` with an accepted host key and a stored password, the display a
//! client asks for, the end row with its audit entries and events, and a capture of the
//! process log (LOG-007).

use std::{
    io::{self, Write as _},
    sync::{Arc, Mutex},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    credentials::{self, Secret, SshSecret, Vault},
    events::{ClientAddr, Operator},
    ids::{DeviceId, OperatorId, SessionId},
    sessions::{DisplayRequest, Protocol, PublicKey, StartRequest},
};

pub(in crate::sessions) const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const OPERATOR: &str = "0190a000-0000-7000-8000-000000000001";
pub(in crate::sessions) const PASSWORD: &str = "correct horse battery staple";
/// The address the test client connects from, and the device's SSH server.
pub(in crate::sessions) const LOOPBACK: &str = "127.0.0.1";

pub(in crate::sessions) fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

pub(in crate::sessions) fn operator() -> Operator {
    Operator {
        id: OPERATOR.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
    }
}

pub(in crate::sessions) fn address(text: &str) -> ClientAddr {
    ClientAddr(text.parse().unwrap())
}

pub(in crate::sessions) fn docker_host() -> DeviceId {
    DOCKER_HOST.parse().unwrap()
}

/// A vault over a throwaway key in a temporary file (SEC-004).
pub(in crate::sessions) fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// The operator, `accepted` as `docker-host`'s host key, and [`PASSWORD`] for it.
pub(in crate::sessions) async fn prepare(pool: &PgPool, vault: &Vault, accepted: &PublicKey) {
    sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::text::uuid, 'operator', '$argon2id$x', now())",
    )
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into sessions.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1::text::uuid, $2, $3, $4, $5, $5, $6::text::uuid)",
    )
    .bind(DOCKER_HOST)
    .bind(accepted.algorithm())
    .bind(accepted.fingerprint().as_str().to_owned())
    .bind(accepted.to_base64())
    .bind(now())
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
    store_password(pool, vault, PASSWORD).await;
}

/// Stores `password` as `docker-host`'s SSH credential, replacing the one stored.
pub(in crate::sessions) async fn store_password(pool: &PgPool, vault: &Vault, password: &str) {
    let mut conn = pool.acquire().await.unwrap();
    let password = SshSecret::Password(Secret::from(password.to_owned()));
    credentials::store_ssh(&mut conn, vault, docker_host(), password)
        .await
        .unwrap();
}

/// Points `docker-host` at `host` and `port`, as an operator's edit would.
pub(in crate::sessions) async fn move_device(pool: &PgPool, host: &str, port: u16) {
    sqlx::query("update devices.devices set host = $2 where id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(host)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("update devices.ssh_settings set port = $2 where device_id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(i32::from(port))
        .execute(pool)
        .await
        .unwrap();
}

/// A pool of its own on the test database for the route. sqlx test pools share 20 connections
/// across all running tests; the session task writes while the test reads, and two
/// connections per test starve that share when many tests run at once.
pub(in crate::sessions) async fn own_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

/// The SSH session a client asks for on `docker-host`.
pub(in crate::sessions) fn request() -> StartRequest {
    StartRequest {
        device: docker_host(),
        protocol: Protocol::Ssh,
        display: DisplayRequest {
            width_px: 1280,
            height_px: 800,
            dpi: 96,
            timezone: Some("Europe/Berlin".to_owned()),
        },
    }
}

/// What a session left behind once its row ended.
#[derive(Debug)]
pub(in crate::sessions) struct Ended {
    pub(in crate::sessions) reason: String,
    /// `(operator name, client address, change)` of each `session.end` entry.
    pub(in crate::sessions) audits: Vec<(String, String, Value)>,
    /// `(severity, source, device name, message)` of each event.
    pub(in crate::sessions) events: Vec<(String, String, Option<String>, String)>,
}

/// Waits up to five seconds for the session's row to end, then reads what the end wrote.
pub(in crate::sessions) async fn ended(pool: &PgPool, session: SessionId) -> Ended {
    for _ in 0..250 {
        let reason: Option<String> =
            sqlx::query_scalar("select end_reason from sessions.sessions where id = $1")
                .bind(session)
                .fetch_one(pool)
                .await
                .unwrap();
        if let Some(reason) = reason {
            // The audit entry and event commit with the row.
            return Ended {
                reason,
                audits: end_audits(pool, session).await,
                events: events(pool).await,
            };
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the session did not end");
}

async fn end_audits(pool: &PgPool, session: SessionId) -> Vec<(String, String, Value)> {
    sqlx::query_as(
        "select operator_name, host(client_addr), change from events.audit_log \
         where action = 'end' and target_type = 'session' and target_id = $1",
    )
    .bind(session.to_string())
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn events(pool: &PgPool) -> Vec<(String, String, Option<String>, String)> {
    sqlx::query_as("select severity, source, device_name, message from events.event_log")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The end row's reason and its one audit entry (by `client`) and one event (`severity`).
pub(in crate::sessions) fn ended_once(ended: &Ended, reason: &str, severity: &str, client: &str) {
    assert_eq!(ended.reason, reason);
    let audit = (
        "operator".to_owned(),
        client.to_owned(),
        serde_json::json!({ "reason": reason }),
    );
    assert_eq!(ended.audits, [audit], "one session.end entry");
    assert_eq!(ended.events.len(), 1, "one event: {:?}", ended.events);
    let (got_severity, source, device, _) = &ended.events[0];
    assert_eq!(
        (got_severity.as_str(), source.as_str()),
        (severity, "session")
    );
    assert_eq!(device.as_deref(), Some("docker-host"));
}

/// Every text the database holds about sessions, audit and events, as one string.
pub(in crate::sessions) async fn stored_text(pool: &PgPool) -> String {
    let rows: Vec<String> = sqlx::query_scalar(
        "select row_to_json(s)::text from sessions.sessions s \
         union all select row_to_json(a)::text from events.audit_log a \
         union all select row_to_json(e)::text from events.event_log e",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    rows.join("\n")
}

/// Collects everything the process log writes.
#[derive(Clone, Default)]
pub(in crate::sessions) struct Capture(pub(in crate::sessions) Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
