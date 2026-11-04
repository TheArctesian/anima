//! `GET /tunnel` (K4 Section 2, API-009): every refusal happens before the WebSocket upgrade,
//! with an HTTP status and an empty body.
//!
//! The order of the checks:
//!
//! 1. The lazy sweep of expired tickets (D-2); if it cannot run, 503 before any token is read.
//! 2. The token from the query and the client address, then `Live::redeem` (403). A presented
//!    token is spent whatever happens next, valid or not (K4 Section 1), so the ticket is
//!    redeemed before the subprotocol is checked: a request without `guacamole` still spends
//!    it, and cannot be used to test a token without spending it.
//! 3. A WebSocket upgrade offering the subprotocol `guacamole` (400). The ticket is already
//!    redeemed, so the session ends `PROTOCOL_ERROR`.
//! 4. The device, the host target, the host-key check and the guacd handshake (`opening`);
//!    a failure is 503 and the session ends `GATEWAY_ERROR`.
//!
//! Neither the URI, the query nor the token reaches a log, a span or an error (LOG-007).

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{
        RawQuery, State, WebSocketUpgrade,
        ws::{WebSocket, rejection::WebSocketUpgradeRejection},
    },
    http::{Extensions, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use tokio::sync::oneshot;
use zeroize::Zeroizing;

use super::{
    ending::{CLIENT_IDLE, Cause, End, GATEWAY_SILENCE},
    opening::{self, Opening, ReadHostKey},
    record::finish,
    relaying,
};
use crate::{
    access,
    clock::Clock,
    credentials::Vault,
    events::Bus,
    sessions::{Live, Redeemed, end::sweep},
};

/// The only path of the tunnel in Phase 1 (K4 Section 1 `tunnelPath`).
const PATH: &str = "/tunnel";
/// The WebSocket subprotocol guacamole-common-js asks for (K4 Section 2).
const SUBPROTOCOL: &str = "guacamole";
/// Largest WebSocket message and frame from the client (WP-08 H-3): eight of guacd's largest
/// instructions, so one message never decodes into an unbounded number of instructions.
pub(super) const MAX_MESSAGE_BYTES: usize = 64 * 1024;
/// How long the hub waits for each of: resolving the device host, reading its host key,
/// reaching guacd, and the guacd handshake (K4 Section 3, D-7).
const CONNECT_WAIT: Duration = Duration::from_secs(10);

/// The waits of one tunnel. Tests shorten them; the hub uses `Timing::default()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Timing {
    /// Each of: resolving the device host, reading its host key, connecting to guacd.
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
            connect: CONNECT_WAIT,
            handshake: CONNECT_WAIT,
            gateway_silence: GATEWAY_SILENCE,
            client_idle: CLIENT_IDLE,
        }
    }
}

/// What the tunnel route needs: the hub's stores, the gateway address, the live state, the
/// waits and the host-key reader.
#[derive(Clone)]
pub(super) struct Route {
    pub(super) db: PgPool,
    pub(super) vault: Vault,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    /// guacd's `host:port` (K7, R-032 E-8).
    pub(super) guacd: String,
    pub(super) live: Live,
    pub(super) timing: Timing,
    /// The hub's own read of the device's host key before the handshake (R-034); `None`
    /// skips the read and leaves the check to guacd's pin alone.
    pub(super) read_host_key: Option<ReadHostKey>,
}

/// The `/tunnel` route (K3, K4 Section 2). `main.rs` mounts it behind
/// `access::permitted_networks_layer` (NET-002) and serves it with `ConnectInfo<SocketAddr>`,
/// so the client address is known.
///
/// Until WP-10 T3's probe is wired in, the hub does not read the host key itself; guacd still
/// pins the accepted key (`host-key`), so a changed key fails closed as `GATEWAY_ERROR`.
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
    live: Live,
) -> Router {
    router(Route {
        db,
        vault,
        bus,
        clock,
        guacd,
        live,
        timing: Timing::default(),
        read_host_key: None,
    })
}

/// The route over `route`'s dependencies, waits and host-key reader.
pub(super) fn router(route: Route) -> Router {
    Router::new()
        .route(PATH, get(open))
        .with_state(Arc::new(route))
}

/// The value of the first `token=` pair. No decoding: a token is base64url, so a
/// percent-encoded one is not canonical and is refused like any unknown token.
fn token_in(query: &str) -> Option<&str> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("token="))
}

/// The upgrade, when it is one and offers the `guacamole` subprotocol, with the size caps.
fn guacamole(
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Option<WebSocketUpgrade> {
    let upgrade = upgrade.ok()?.protocols([SUBPROTOCOL]);
    upgrade.selected_protocol()?;
    Some(
        upgrade
            .max_message_size(MAX_MESSAGE_BYTES)
            .max_frame_size(MAX_MESSAGE_BYTES),
    )
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
    let Some(upgrade) = guacamole(upgrade) else {
        let end = End::explained(
            Cause::InvalidInstruction,
            "The client did not open the tunnel as a Guacamole WebSocket, so the hub closed \
             the SSH session.",
        );
        finish(&route, redeemed, &end).await;
        return StatusCode::BAD_REQUEST.into_response();
    };
    match opening::open(&route, &redeemed.ticket).await {
        Ok(opening) => upgrade_to(route, redeemed, upgrade, opening),
        Err(not_opened) => {
            let session = redeemed.ticket.session;
            tracing::warn!(%session, error = %not_opened, "the tunnel did not open");
            let end = End::explained(Cause::GatewayError, not_opened.message());
            finish(&route, redeemed, &end).await;
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
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
    if let Err(error) = sweep(&route.db, &route.live, now).await {
        tracing::error!(%error, "the tunnel could not sweep expired tickets");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let (Some(token), Some(client)) = (token_in(query), access::client_addr(extensions)) else {
        return Err(StatusCode::FORBIDDEN);
    };
    route
        .live
        .redeem(token, client, now)
        .map_err(|_| StatusCode::FORBIDDEN)
}

/// Answers with the upgrade and hands the socket to the tunnel task, which owns the session
/// from here: if the upgrade fails, the task sees no socket and still ends the session.
fn upgrade_to(
    route: Arc<Route>,
    redeemed: Redeemed,
    upgrade: WebSocketUpgrade,
    opening: Opening,
) -> Response {
    let (socket_tx, socket_rx) = oneshot::channel::<WebSocket>();
    let response = upgrade.on_upgrade(move |socket| async move {
        // Unsent only when the tunnel task has already ended (shutdown, or the upgrade wait
        // ran out); the socket then closes as it drops.
        let _ = socket_tx.send(socket);
    });
    tokio::spawn(relaying::run(route, redeemed, opening, socket_rx));
    response
}
