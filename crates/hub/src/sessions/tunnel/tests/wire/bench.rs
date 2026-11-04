//! A hub serving only `/tunnel` on a loopback port, with a started session on `docker-host`
//! (`10.10.0.21`, a K6 device whose host is an IP literal, so the target rule runs as in
//! production without DNS), and what a session leaves in the database.

use std::{io::Write as _, net::SocketAddr, sync::Arc, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;

use super::super::super::route::{Route, Timing, router};
use crate::{
    clock::{Clock, FixedClock},
    credentials::{self, Secret, SshSecret, Vault},
    events::{Bus, ClientAddr, Operator},
    ids::{DeviceId, OperatorId, SessionId},
    sessions::{DisplayRequest, Live, Protocol, PublicKey, StartRequest, start, tests::ED25519},
};

pub(super) const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const OPERATOR: &str = "0190a000-0000-7000-8000-000000000001";
pub(super) const PASSWORD: &str = "correct horse battery staple";
/// The address the test client connects from.
pub(super) const LOOPBACK: &str = "127.0.0.1";

pub(super) fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

pub(super) fn operator() -> Operator {
    Operator {
        id: OPERATOR.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
    }
}

pub(super) fn address(text: &str) -> ClientAddr {
    ClientAddr(text.parse().unwrap())
}

/// Waits long enough that only the test's own action ends a session.
pub(super) fn patient() -> Timing {
    Timing {
        connect: Duration::from_secs(3),
        handshake: Duration::from_secs(3),
        gateway_silence: Duration::from_secs(60),
        client_idle: Duration::from_secs(60),
    }
}

/// How the bench differs from the default: the route's waits, host-key reader and clock,
/// and the address the ticket is bound to.
pub(super) struct Setup {
    pub(super) route: Box<dyn FnOnce(&mut Route) + Send>,
    pub(super) ticket_client: ClientAddr,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            route: Box::new(|_| {}),
            ticket_client: address(LOOPBACK),
        }
    }
}

impl Setup {
    pub(super) fn route(tune: impl FnOnce(&mut Route) + Send + 'static) -> Self {
        Self {
            route: Box::new(tune),
            ..Self::default()
        }
    }
}

/// A served tunnel route and one started session's ticket.
pub(super) struct Bench {
    pub(super) live: Live,
    pub(super) session: SessionId,
    pub(super) token: String,
    pub(super) url: String,
}

/// A vault over a throwaway key in a temporary file (SEC-004).
fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// The operator, an accepted ed25519 host key and a password for `docker-host`.
async fn prepare(pool: &PgPool, vault: &Vault) {
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
         values ($1::text::uuid, 'ssh-ed25519', $2, $3, $4, $4, $5::text::uuid)",
    )
    .bind(DOCKER_HOST)
    .bind(
        PublicKey::from_base64(ED25519)
            .unwrap()
            .fingerprint()
            .as_str()
            .to_owned(),
    )
    .bind(ED25519)
    .bind(now())
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let password = SshSecret::Password(Secret::from(PASSWORD.to_owned()));
    credentials::store_ssh(&mut conn, vault, docker_host(), password)
        .await
        .unwrap();
}

pub(super) fn docker_host() -> DeviceId {
    DOCKER_HOST.parse().unwrap()
}

/// A pool of its own on the test database for the route. sqlx test pools share 20 connections
/// across all running tests; the tunnel task writes while the test reads, and two
/// connections per test starve that share when many tests run at once.
async fn own_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

/// Starts a session on `docker-host` at `now()` and serves the tunnel route to `guacd`.
pub(super) async fn bench(pool: &PgPool, guacd: &str, setup: Setup) -> Bench {
    let vault = vault();
    prepare(pool, &vault).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let live = Live::default();
    let request = StartRequest {
        device: docker_host(),
        protocol: Protocol::Ssh,
        display: DisplayRequest {
            width_px: 1280,
            height_px: 800,
            dpi: 96,
            timezone: Some("Europe/Berlin".to_owned()),
        },
    };
    let ticket = start(
        pool,
        &clock,
        &live,
        &operator(),
        setup.ticket_client,
        request,
    )
    .await
    .unwrap();
    let mut route = Route {
        db: own_pool(pool).await,
        vault,
        bus: Bus::new(16),
        clock,
        guacd: guacd.to_owned(),
        live: live.clone(),
        timing: patient(),
        read_host_key: None,
    };
    (setup.route)(&mut route);
    let listener = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let served = listener.local_addr().unwrap();
    let app = router(route).into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Bench {
        live,
        session: ticket.session,
        token: ticket.token().to_owned(),
        url: format!("ws://{served}/tunnel?token={}", ticket.token()),
    }
}

/// A route clock `seconds` after the ticket was issued.
pub(super) fn clock_at(seconds: i64) -> Arc<dyn Clock> {
    Arc::new(FixedClock(now() + TimeDelta::seconds(seconds)))
}

/// What a session left behind once its row ended.
#[derive(Debug)]
pub(super) struct Ended {
    pub(super) reason: String,
    /// `(operator name, client address, change)` of each `session.end` entry.
    pub(super) audits: Vec<(String, String, Value)>,
    /// `(severity, source, device name, message)` of each event.
    pub(super) events: Vec<(String, String, Option<String>, String)>,
}

/// Waits up to five seconds for the session's row to end, then reads what the end wrote.
pub(super) async fn ended(pool: &PgPool, session: SessionId) -> Ended {
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

/// Every text the database holds about sessions, audit and events, as one string.
pub(super) async fn stored_text(pool: &PgPool) -> String {
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
