//! A hub serving only `/tunnel` on a loopback port, with a started RDP or VNC session on
//! `docker-host`. The session starts while the device's host is `10.10.0.21` (a K6 device),
//! then the device moves to loopback, which the tests' target rule lets the tunnel reach.
//! Its RDP port is a fake RDP server's, whose certificate is the accepted one unless the
//! setup says otherwise.

mod fake_rdp;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::net::TcpListener;

use super::super::super::route::{Route, Timing, router};
use crate::{
    clock::{Clock, FixedClock},
    credentials::{self, PasswordKind, Secret, Vault},
    events::{Bus, ClientAddr},
    ids::SessionId,
    sessions::{
        DisplayRequest, Live, Presented, Protocol, StartRequest, start,
        tests::fixture::{DOCKER_HOST, add_operator, own_pool, vault},
    },
};
use fake_rdp::{Answer, FakeRdp, fake_rdp};

pub(super) use crate::sessions::tests::fixture::{
    LOOPBACK, PASSWORD, address, docker_host, now, operator,
};

/// `docker-host`'s VNC password; [`PASSWORD`] is its RDP password.
pub(super) const VNC_PASSWORD: &str = "vnc tr0ub4dor";
/// An accepted fingerprint that no fake RDP server presents.
pub(super) const FINGERPRINT: &str = "0a:1b:2c:3d:4e:5f:60:71:82:93:a4:b5:c6:d7:e8:f9:\
                                      0a:1b:2c:3d:4e:5f:60:71:82:93:a4:b5:c6:d7:e8:f9";
/// Where the route tells guacd to keep shared drives.
pub(super) const DRIVE_ROOT: &str = "/drives";

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

/// What the device's RDP server presents when the opening reads its certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Presents {
    /// The accepted certificate.
    Accepted,
    /// Another certificate than the accepted one.
    Changed,
    /// No certificate: it offers standard RDP security only.
    StandardOnly,
}

/// How the bench differs from the default: the route's waits and clock, the live state and
/// its target rule, the address the ticket is bound to, the session's protocol and what the
/// device's RDP server presents.
pub(super) struct Setup {
    pub(super) route: Box<dyn FnOnce(&mut Route) + Send>,
    pub(super) ticket_client: ClientAddr,
    pub(super) live: Live,
    pub(super) protocol: Protocol,
    pub(super) presents: Presents,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            route: Box::new(|_| {}),
            ticket_client: address(LOOPBACK),
            live: Live::default().reaching_loopback_for_tests(),
            protocol: Protocol::Rdp,
            presents: Presents::Accepted,
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

    pub(super) fn protocol(protocol: Protocol) -> Self {
        Self {
            protocol,
            ..Self::default()
        }
    }

    pub(super) fn presents(presents: Presents) -> Self {
        Self {
            presents,
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
    /// The device's RDP server.
    pub(super) rdp: FakeRdp,
    /// The fingerprint of the device's accepted RDP server certificate.
    pub(super) accepted: String,
}

/// Turns RDP (NLA on `rdp_port`, user `ops` in `LAB`) and VNC (user `viewer`) on for
/// `docker-host`, accepts the RDP server certificate whose fingerprint is `accepted` and
/// stores both passwords.
pub(super) async fn desktop_ready(pool: &PgPool, vault: &Vault, rdp_port: u16, accepted: &str) {
    add_operator(pool).await;
    sqlx::query(
        "update devices.rdp_settings set enabled = true, username = 'ops', domain = 'LAB', \
         port = $2 where device_id = $1::text::uuid",
    )
    .bind(DOCKER_HOST)
    .bind(i32::from(rdp_port))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "update devices.vnc_settings set enabled = true, username = 'viewer' \
         where device_id = $1::text::uuid",
    )
    .bind(DOCKER_HOST)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into sessions.server_certificates (device_id, fingerprint_sha256, \
                first_seen_at, accepted_by) \
         select $1::text::uuid, $2, $3, id from access.operators",
    )
    .bind(DOCKER_HOST)
    .bind(accepted)
    .bind(now())
    .execute(pool)
    .await
    .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    for (kind, password) in [
        (PasswordKind::Rdp, PASSWORD),
        (PasswordKind::Vnc, VNC_PASSWORD),
    ] {
        let secret = Secret::from(password.to_owned());
        credentials::store_password(&mut conn, vault, docker_host(), kind, secret)
            .await
            .unwrap();
    }
}

/// The session a client asks for on `docker-host` over `protocol`.
pub(super) fn request(protocol: Protocol) -> StartRequest {
    StartRequest {
        device: docker_host(),
        protocol,
        display: DisplayRequest {
            width_px: 1280,
            height_px: 800,
            dpi: 96,
            timezone: Some("Europe/Berlin".to_owned()),
        },
    }
}

/// Points `docker-host` at `host`, as an operator's edit would.
pub(super) async fn move_host(pool: &PgPool, host: &str) {
    sqlx::query("update devices.devices set host = $2 where id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(host)
        .execute(pool)
        .await
        .unwrap();
}

/// Starts a fake RDP server and a session on `docker-host` at `now()`, moves the device to
/// loopback, and serves the tunnel route to `guacd`.
pub(super) async fn bench(pool: &PgPool, guacd: &str, setup: Setup) -> Bench {
    let vault = vault();
    let answer = if setup.presents == Presents::StandardOnly {
        Answer::Standard
    } else {
        Answer::Nla
    };
    let rdp = fake_rdp(answer).await;
    let accepted = if setup.presents == Presents::Accepted {
        rdp.fingerprint.clone()
    } else {
        FINGERPRINT.to_owned()
    };
    desktop_ready(pool, &vault, rdp.port, &accepted).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let live = setup.live;
    let client = setup.ticket_client;
    let ticket = start(
        pool,
        &clock,
        &live,
        &operator(),
        client,
        request(setup.protocol),
    )
    .await
    .unwrap();
    move_host(pool, LOOPBACK).await;
    let mut route = Route {
        db: own_pool(pool).await,
        vault,
        bus: Bus::new(16),
        clock,
        guacd: guacd.to_owned(),
        drive_root: DRIVE_ROOT.to_owned(),
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
        rdp,
        accepted,
    }
}

/// `token` from the test client on `/tunnel`, as guacamole-common-js presents it.
pub(super) fn on_tunnel(token: &str) -> Presented<'_> {
    Presented {
        path: "/tunnel",
        ..Presented::terminal(token, address(LOOPBACK))
    }
}

/// A route clock `seconds` after the ticket was issued.
pub(super) fn clock_at(seconds: i64) -> Arc<dyn Clock> {
    Arc::new(FixedClock(now() + TimeDelta::seconds(seconds)))
}
