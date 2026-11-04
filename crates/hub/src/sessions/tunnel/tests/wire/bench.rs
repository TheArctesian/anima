//! A hub serving only `/tunnel` on a loopback port, with a started session on `docker-host`.
//! The session starts while the device's host is `10.10.0.21` (a K6 device), then the device
//! moves to an SSH server on loopback, which the tests' target rule lets the probe reach.

use std::{io::Write as _, net::SocketAddr, sync::Arc, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;

use super::{
    super::super::route::{Route, Timing, router},
    ssh_host::ssh_host,
};
use crate::{
    clock::{Clock, FixedClock},
    credentials::{self, Secret, SshSecret, Vault},
    events::{Bus, ClientAddr, Operator},
    ids::{DeviceId, OperatorId, SessionId},
    sessions::{DisplayRequest, Live, Protocol, PublicKey, StartRequest, start},
};

pub(super) const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const OPERATOR: &str = "0190a000-0000-7000-8000-000000000001";
pub(super) const PASSWORD: &str = "correct horse battery staple";
/// The address the test client connects from, and the device's SSH server.
pub(super) const LOOPBACK: &str = "127.0.0.1";
/// The seed of the host key the device's SSH server presents.
const HOST_KEY_SEED: u8 = 3;

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

/// How long the tests' `endSession` waits for a tunnel to end its row (D-10): long enough
/// that a loaded machine still finishes, far short of the waits in [`patient`], so only the
/// end signal ends a session within it. These tests run on real sockets and PostgreSQL, so
/// paused tokio time cannot stand in: auto-advance would fire the hub's waits while real I/O
/// is still pending.
pub(super) const END_WAIT: Duration = Duration::from_secs(10);

/// The default setup with `endSession` waiting up to [`END_WAIT`].
pub(super) fn ending_within_end_wait() -> Setup {
    Setup {
        live: Live::with_tunnel_end_wait(END_WAIT).reaching_loopback_for_tests(),
        ..Setup::default()
    }
}

/// How the bench differs from the default: the route's waits and clock, the live state and
/// its target rule, the address the ticket is bound to, and the accepted host key when it is
/// not the one the device presents.
pub(super) struct Setup {
    pub(super) route: Box<dyn FnOnce(&mut Route) + Send>,
    pub(super) ticket_client: ClientAddr,
    pub(super) live: Live,
    pub(super) accepted: Option<PublicKey>,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            route: Box::new(|_| {}),
            ticket_client: address(LOOPBACK),
            live: Live::default().reaching_loopback_for_tests(),
            accepted: None,
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
    /// The route as served, for tests that drive the session task's parts directly.
    pub(super) route: Route,
    pub(super) session: SessionId,
    pub(super) token: String,
    pub(super) url: String,
    /// Where the route listens.
    pub(super) served: SocketAddr,
    /// The device's SSH port and the host key it presents.
    pub(super) ssh_port: u16,
    pub(super) host_key: PublicKey,
}

/// A vault over a throwaway key in a temporary file (SEC-004).
fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// The operator, `accepted` as `docker-host`'s host key, and a password for it.
async fn prepare(pool: &PgPool, vault: &Vault, accepted: &PublicKey) {
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
    let mut conn = pool.acquire().await.unwrap();
    let password = SshSecret::Password(Secret::from(PASSWORD.to_owned()));
    credentials::store_ssh(&mut conn, vault, docker_host(), password)
        .await
        .unwrap();
}

pub(super) fn docker_host() -> DeviceId {
    DOCKER_HOST.parse().unwrap()
}

/// Points `docker-host` at `host` and `port`, as an operator's edit would.
pub(super) async fn move_device(pool: &PgPool, host: &str, port: u16) {
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
/// across all running tests; the tunnel task writes while the test reads, and two
/// connections per test starve that share when many tests run at once.
async fn own_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

/// The display a client asks for.
fn request() -> StartRequest {
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

/// Starts a session on `docker-host` at `now()`, moves the device to an SSH server on
/// loopback, and serves the tunnel route to `guacd`.
pub(super) async fn bench(pool: &PgPool, guacd: &str, setup: Setup) -> Bench {
    let vault = vault();
    let ssh = ssh_host(HOST_KEY_SEED).await;
    prepare(pool, &vault, setup.accepted.as_ref().unwrap_or(&ssh.key)).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let live = setup.live;
    let client = setup.ticket_client;
    let ticket = start(pool, &clock, &live, &operator(), client, request())
        .await
        .unwrap();
    move_device(pool, LOOPBACK, ssh.port).await;
    let mut route = Route {
        db: own_pool(pool).await,
        vault,
        bus: Bus::new(16),
        clock,
        guacd: guacd.to_owned(),
        live: live.clone(),
        timing: patient(),
    };
    (setup.route)(&mut route);
    let listener = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let served = listener.local_addr().unwrap();
    let app = router(route.clone()).into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Bench {
        live,
        route,
        session: ticket.session,
        token: ticket.token().to_owned(),
        url: format!("ws://{served}/tunnel?token={}", ticket.token()),
        served,
        ssh_port: ssh.port,
        host_key: ssh.key,
    }
}

/// A route clock `seconds` after the ticket was issued.
pub(super) fn clock_at(seconds: i64) -> Arc<dyn Clock> {
    Arc::new(FixedClock(now() + TimeDelta::seconds(seconds)))
}
