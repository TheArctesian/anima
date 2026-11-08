//! `GET /terminal` (K4 Section 8.1, API-009): every refusal happens before the WebSocket
//! upgrade, with an HTTP status and an empty body.
//!
//! The order of the checks (`admission`), as on the Guacamole tunnel:
//!
//! 1. The lazy sweep of expired tickets (D-2); if it cannot run, 503 before any token is read.
//! 2. The token from the query, the client address and the path, then `Live::redeem` (403;
//!    a ticket whose `tunnelPath` is not `/terminal` is refused and spent too). The token is
//!    spent before the subprotocol is checked (K4 Section 1).
//! 3. From here the session task owns the redeemed ticket (`upgrade`), so the session ends
//!    exactly once even when the client leaves and this handler is dropped (MV-003). The task
//!    answers this handler: 400 without the subprotocol `anima-terminal` (the session ends
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

use crate::{
    clock::Clock,
    credentials::Vault,
    events::Bus,
    sessions::{
        Live, PendingTicket, Protocol, admission,
        end::CLIENT_IDLE,
        tickets::{RECONNECT, WOKEN_OPENING},
        upgrade::run_session,
    },
    ssh::probe::PROBE_TIMEOUT,
};

/// The terminal tunnel's path (K4 Section 1 `tunnelPath`): SSH's.
const PATH: &str = Protocol::Ssh.tunnel_path();
/// The WebSocket subprotocol the client asks for (K4 Section 8.1).
const SUBPROTOCOL: &str = "anima-terminal";
/// How long the hub waits for the upgrade after it answered, and for the client's first
/// `resize` after the upgrade (K4 Section 8.5).
const CLIENT_WAIT: Duration = Duration::from_secs(10);
/// The hub sends `ping` when it has sent the client nothing for this long (K4 Section 8.4).
const HUB_SILENCE: Duration = Duration::from_secs(5);
/// The longest one write or request to the device may wait (K4 Section 8.5).
const DEVICE_WAIT: Duration = Duration::from_secs(15);

/// The waits of one terminal session (K4 Section 8.5). Tests shorten them; the hub uses
/// `Timing::default()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Timing {
    /// The whole opening: lookup, connect, key exchange, sign-in and the session channel.
    pub(super) opening: Duration,
    /// The whole opening of a ticket whose start sent a Wake-on-LAN packet (K4 Section 1).
    pub(super) woken_opening: Duration,
    /// How often that opening tries to connect again while the device does not answer.
    pub(super) reconnect: Duration,
    /// The WebSocket upgrade after the hub answered.
    pub(super) upgrade: Duration,
    /// The client's first `resize` after the upgrade.
    pub(super) first_size: Duration,
    /// Hub silence after which the client gets a `ping`.
    pub(super) hub_silence: Duration,
    /// Client silence after which the session ends `TIMEOUT`, not counting time the client's
    /// input waits on the device; also the longest one send to the client may take.
    pub(super) client_idle: Duration,
    /// The longest one write of client input to the device, or one PTY, shell or
    /// window-change request with its answer, may take before the session ends
    /// `GATEWAY_ERROR`.
    pub(super) request: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            opening: PROBE_TIMEOUT,
            woken_opening: WOKEN_OPENING,
            reconnect: RECONNECT,
            upgrade: CLIENT_WAIT,
            first_size: CLIENT_WAIT,
            hub_silence: HUB_SILENCE,
            client_idle: CLIENT_IDLE,
            request: DEVICE_WAIT,
        }
    }
}

impl Timing {
    /// The limit of `ticket`'s whole opening: longer when its start woke the device.
    pub(super) const fn opening_of(&self, ticket: &PendingTicket) -> Duration {
        if ticket.wake_sent {
            self.woken_opening
        } else {
            self.opening
        }
    }
}

/// What the terminal route needs: the hub's stores, the live state and the waits.
#[derive(Clone)]
pub(super) struct Terminal {
    pub(super) db: PgPool,
    pub(super) vault: Vault,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    /// The tickets and tunnels, and the target rule for the device's host (R-032).
    pub(super) live: Live,
    pub(super) timing: Timing,
}

/// The `/terminal` route (K3, K4 Section 8.1). `main.rs` mounts it behind
/// `access::permitted_networks_layer` (NET-002) and serves it with `ConnectInfo<SocketAddr>`,
/// so the client address is known.
#[expect(
    clippy::too_many_arguments,
    reason = "K3 fixes terminal_router's parameters (R-062)"
)]
pub fn terminal_router(
    db: PgPool,
    vault: Vault,
    bus: Bus,
    clock: Arc<dyn Clock>,
    live: Live,
) -> Router {
    router(Terminal {
        db,
        vault,
        bus,
        clock,
        live,
        timing: Timing::default(),
    })
}

/// The route over `terminal`'s dependencies and waits, at the path K4 Section 1 gives SSH.
pub(super) fn router(terminal: Terminal) -> Router {
    Router::new()
        .route(PATH, get(open))
        .with_state(Arc::new(terminal))
}

async fn open(
    State(terminal): State<Arc<Terminal>>,
    RawQuery(query): RawQuery,
    extensions: Extensions,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Response {
    let query = Zeroizing::new(query.unwrap_or_default());
    let now = terminal.clock.now();
    if let Err(status) = admission::swept(&terminal.db, &terminal.live, now).await {
        return status.into_response();
    }
    let request = admission::Request {
        query: &query,
        extensions: &extensions,
        now,
    };
    let redeemed = match admission::redeem(&terminal.live, request, PATH, Protocol::tunnel_path) {
        Ok(redeemed) => redeemed,
        Err(status) => return status.into_response(),
    };
    drop(query);
    let upgrade = admission::offering(upgrade, SUBPROTOCOL);
    let (answer, answered) = oneshot::channel();
    // Not awaited here: dropping this handler must not drop the session (MV-003).
    tokio::spawn(run_session(terminal, redeemed, upgrade, answer));
    // No answer only when the task stopped before giving one; it still ended the session.
    answered
        .await
        .unwrap_or_else(|_| StatusCode::SERVICE_UNAVAILABLE.into_response())
}
