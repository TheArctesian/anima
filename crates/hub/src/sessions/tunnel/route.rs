//! `GET /tunnel` (K4 Section 2, API-009): every refusal happens before the WebSocket upgrade,
//! with an HTTP status and an empty body.
//!
//! It serves RDP and VNC tickets only: an SSH ticket belongs to `/terminal` (K4 Section 8,
//! R-062), so it is refused and spent like a ticket for any other path (403).
//!
//! The order of the checks (`admission`):
//!
//! 1. The lazy sweep of expired tickets (D-2); if it cannot run, 503 before any token is read.
//! 2. The token from the query and the client address, then `Live::redeem` (403). A presented
//!    token is spent whatever happens next, valid or not (K4 Section 1), so the ticket is
//!    redeemed before the subprotocol is checked: a request without `guacamole` still spends
//!    it, and cannot be used to test a token without spending it. A ticket whose `tunnelPath`
//!    is not `/tunnel` is refused and spent in the same redemption (403).
//! 3. From here the session task owns the redeemed ticket (`relaying`), so the session ends
//!    exactly once even when the client leaves and this handler is dropped (MV-003). The task
//!    answers this handler: 400 without the subprotocol `guacamole` (the session ends
//!    `PROTOCOL_ERROR`), 503 when the opening fails (`GATEWAY_ERROR`), else the upgrade.
//!
//! Neither the URI, the query nor the token reaches a log, a span or an error (LOG-007).

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{RawQuery, State, WebSocketUpgrade, ws::rejection::WebSocketUpgradeRejection},
    http::{Extensions, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use tokio::sync::oneshot;
use zeroize::Zeroizing;

use super::{
    ending::{CLIENT_IDLE, GATEWAY_SILENCE},
    relaying,
};
use crate::{
    clock::Clock,
    credentials::Vault,
    events::Bus,
    sessions::{Live, Protocol, Redeemed, admission, end::Records, probe::PROBE_TIMEOUT},
};

/// The tunnel's path (K4 Section 1 `tunnelPath` of RDP and VNC).
const PATH: &str = "/tunnel";
/// The WebSocket subprotocol guacamole-common-js asks for (K4 Section 2). Its 64 KiB message
/// cap (`admission`) is eight of guacd's largest instructions, so one message never decodes
/// into an unbounded number of instructions (WP-08 H-3).
const SUBPROTOCOL: &str = "guacamole";
/// How long the hub waits for the guacd handshake and for the upgrade after it (K4 Section 3,
/// D-7).
const HANDSHAKE_WAIT: Duration = Duration::from_secs(10);

/// The waits of one tunnel. Tests shorten them; the hub uses `Timing::default()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Timing {
    /// Each of: resolving the device host, reading its RDP server certificate, connecting to
    /// guacd.
    pub(super) connect: Duration,
    /// The guacd handshake, and the WebSocket upgrade after it.
    pub(super) handshake: Duration,
    /// guacd silence after which the client gets a `nop`.
    pub(super) gateway_silence: Duration,
    /// Client silence after which the session ends `TIMEOUT`; also the longest a write to
    /// either side may take.
    pub(super) client_idle: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            connect: PROBE_TIMEOUT,
            handshake: HANDSHAKE_WAIT,
            gateway_silence: GATEWAY_SILENCE,
            client_idle: CLIENT_IDLE,
        }
    }
}

/// What the tunnel route needs: the hub's stores, the gateway and its drive root, the live
/// state and the waits.
#[derive(Clone)]
pub(super) struct Route {
    pub(super) db: PgPool,
    pub(super) vault: Vault,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    /// guacd's `host:port` (K7, R-032 E-8).
    pub(super) guacd: String,
    /// `ANIMA_GUACD_DRIVE_ROOT`: where guacd keeps each device's shared drive (K7, RA-021).
    pub(super) drive_root: String,
    /// The tickets and tunnels, and the target rule for the device's host (R-032).
    pub(super) live: Live,
    pub(super) timing: Timing,
}

impl Route {
    /// Where this route's sessions write their end.
    pub(super) fn records(&self) -> Records {
        Records {
            db: self.db.clone(),
            bus: self.bus.clone(),
            clock: Arc::clone(&self.clock),
        }
    }
}

/// The `/tunnel` route for RDP and VNC sessions (K3, K4 Section 2), with guacd at `guacd`
/// keeping shared drives under `drive_root`. `main.rs` mounts it behind
/// `access::permitted_networks_layer` (NET-002) and serves it with `ConnectInfo<SocketAddr>`,
/// so the client address is known.
#[expect(
    clippy::too_many_arguments,
    reason = "K3 fixes tunnel_router's parameters (R-032 E-1)"
)]
pub fn tunnel_router(
    db: PgPool,
    vault: Vault,
    bus: Bus,
    clock: Arc<dyn Clock>,
    guacd: String,
    drive_root: String,
    live: Live,
) -> Router {
    router(Route {
        db,
        vault,
        bus,
        clock,
        guacd,
        drive_root,
        live,
        timing: Timing::default(),
    })
}

/// The route over `route`'s dependencies and waits.
pub(super) fn router(route: Route) -> Router {
    Router::new()
        .route(PATH, get(open))
        .with_state(Arc::new(route))
}

async fn open(
    State(route): State<Arc<Route>>,
    RawQuery(query): RawQuery,
    extensions: Extensions,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Response {
    let query = Zeroizing::new(query.unwrap_or_default());
    let redeemed = match redeem(&route, &query, &extensions).await {
        Ok(redeemed) => redeemed,
        Err(status) => return status.into_response(),
    };
    drop(query);
    let (answer, answered) = oneshot::channel();
    // Not awaited here: dropping this handler must not drop the session (MV-003).
    let upgrade = admission::offering(upgrade, SUBPROTOCOL);
    tokio::spawn(relaying::run(route, redeemed, upgrade, answer));
    // No answer only when the task stopped before giving one; it still ended the session.
    answered
        .await
        .unwrap_or_else(|_| StatusCode::SERVICE_UNAVAILABLE.into_response())
}

/// Sweeps expired tickets, then redeems the query's token for the client address. The
/// status is 503 when the sweep fails and 403 for every refusal, with no detail (K4
/// Section 2).
async fn redeem(
    route: &Route,
    query: &str,
    extensions: &Extensions,
) -> Result<Redeemed, StatusCode> {
    let now = route.clock.now();
    admission::swept(&route.db, &route.live, now).await?;
    let request = admission::Request {
        query,
        extensions,
        now,
    };
    admission::redeem(&route.live, request, PATH, Protocol::tunnel_path)
}
