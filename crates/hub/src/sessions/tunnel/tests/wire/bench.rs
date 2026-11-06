//! A hub serving only `/tunnel` on a loopback port, with a started session on `docker-host`.
//! The session starts while the device's host is `10.10.0.21` (a K6 device), then the device
//! moves to an SSH server on loopback, which the tests' target rule lets the probe reach.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::net::TcpListener;

use super::{
    super::super::route::{Route, Timing, router},
    ssh_host::ssh_host,
};
use crate::{
    clock::{Clock, FixedClock},
    events::{Bus, ClientAddr},
    ids::SessionId,
    sessions::{
        Live, PublicKey, start,
        tests::fixture::{own_pool, prepare, request, vault},
    },
};

pub(super) use crate::sessions::tests::fixture::{
    LOOPBACK, PASSWORD, address, docker_host, move_device, now, operator,
};
/// The seed of the host key the device's SSH server presents.
const HOST_KEY_SEED: u8 = 3;

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
