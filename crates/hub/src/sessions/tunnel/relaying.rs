//! The session task (K4 Sections 2 to 5): one task per redeemed ticket that opens the gateway
//! side, answers the `/tunnel` request, waits for the upgraded socket, runs the tunnel
//! (`link`) and ends the session once.
//!
//! The task, not the request handler, owns the ticket: hyper drops the handler when the
//! client goes, and the session must still end (MV-003). It holds the ticket in an
//! [`Unfinished`] guard, so even a panic ends the row.

mod link;
mod writes;

use std::sync::Arc;

use axum::{
    extract::{WebSocketUpgrade, ws::WebSocket},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::{sync::oneshot, time::timeout};

use super::{
    ending::{Cause, End},
    gateway::host_key_changed,
    opening::{self, Opening},
    record::Unfinished,
    route::{Route, Timing},
};
use crate::sessions::{Redeemed, Tunnel};
use writes::{close_client, leave_gateway, send_text};

/// The event message when the request did not offer the `guacamole` subprotocol.
const NOT_GUACAMOLE: &str = "The client did not open the tunnel as a Guacamole WebSocket, so \
                             the hub closed the SSH session.";
/// The event message when the client left while the hub opened the gateway side.
const CLIENT_LEFT: &str = "The client left before the SSH session opened.";

/// Runs the session of `redeemed` to its end and writes the end. `upgrade` is the request's
/// upgrade when it offered `guacamole`; `answer` takes the handler's response.
pub(super) async fn run(
    route: Arc<Route>,
    redeemed: Redeemed,
    upgrade: Option<WebSocketUpgrade>,
    answer: oneshot::Sender<Response>,
) {
    let mut unfinished = Unfinished::new(Arc::clone(&route), redeemed);
    let end = session(&route, unfinished.redeemed(), upgrade, answer).await;
    unfinished.finish(&end).await;
}

/// The session up to its first cause: refusals before the upgrade answer 400 or 503.
async fn session(
    route: &Route,
    redeemed: &mut Redeemed,
    upgrade: Option<WebSocketUpgrade>,
    mut answer: oneshot::Sender<Response>,
) -> End {
    let Some(upgrade) = upgrade else {
        let _ = answer.send(StatusCode::BAD_REQUEST.into_response());
        return End::explained(Cause::InvalidInstruction, NOT_GUACAMOLE);
    };
    let opening = match opened(route, redeemed, &mut answer).await {
        Ok(opening) => opening,
        Err(end) => {
            let _ = answer.send(StatusCode::SERVICE_UNAVAILABLE.into_response());
            return end;
        }
    };
    let (socket_tx, socket_rx) = oneshot::channel::<WebSocket>();
    let response = upgrade.on_upgrade(move |socket| async move {
        // Unsent only when the session has already ended; the socket closes as it drops.
        let _ = socket_tx.send(socket);
    });
    // Unanswered when the client has left: the upgrade then never comes, and `connected`
    // ends the session.
    let _ = answer.send(response);
    connected(route.timing, &mut redeemed.tunnel, opening, socket_rx).await
}

/// The opening, unless the session ends first: `endSession` or shutdown (SEC-002, D-10; the
/// opening is dropped before the credential leaves the hub when it can be), or the client
/// leaving, seen as the request handler dropped.
async fn opened(
    route: &Route,
    redeemed: &mut Redeemed,
    answer: &mut oneshot::Sender<Response>,
) -> Result<Opening, End> {
    let Redeemed { ticket, tunnel } = redeemed;
    let session = ticket.session;
    tokio::select! {
        opened = opening::open(route, ticket) => opened.map_err(|not_opened| {
            tracing::warn!(%session, error = %not_opened, "the tunnel did not open");
            End::explained(Cause::GatewayError, not_opened.message())
        }),
        signal = tunnel.ended() => Err(End::from(signal)),
        () = answer.closed() => Err(End::explained(Cause::OperatorClosed, CLIENT_LEFT)),
    }
}

/// The tunnel after the response: waits for the upgraded socket, then relays or reports the
/// changed host key. A host-key mismatch is the cause even if the upgrade fails: it came
/// first (K4 Section 6).
pub(super) async fn connected(
    timing: Timing,
    tunnel: &mut Tunnel,
    opening: Opening,
    socket: oneshot::Receiver<WebSocket>,
) -> End {
    let client = match upgraded(tunnel, socket, timing).await {
        Ok(client) => client,
        Err(end) => {
            return match opening {
                Opening::HostKeyChanged => Cause::HostKeyMismatch.into(),
                Opening::Gateway(connected) => {
                    leave_gateway(connected.stream).await;
                    end
                }
            };
        }
    };
    match opening {
        Opening::HostKeyChanged => host_key_mismatch(client, tunnel, timing).await,
        Opening::Gateway(connected) => link::relay(client, connected, tunnel, timing).await,
    }
}

/// Tells the client the host key changed and closes 4001. guacd was never reached (K4
/// Section 6); the mismatch is the cause even if the client is gone before it hears of it.
async fn host_key_mismatch(mut client: WebSocket, tunnel: &mut Tunnel, timing: Timing) -> End {
    let _ = send_text(&mut client, &host_key_changed(), tunnel, timing.client_idle).await;
    close_client(client, Cause::HostKeyMismatch).await;
    Cause::HostKeyMismatch.into()
}

/// The upgraded socket, unless the session ends first: the end signal, a failed upgrade (the
/// client left) or no upgrade within the handshake wait.
async fn upgraded(
    tunnel: &mut Tunnel,
    socket: oneshot::Receiver<WebSocket>,
    timing: Timing,
) -> Result<WebSocket, End> {
    tokio::select! {
        signal = tunnel.ended() => Err(End::from(signal)),
        socket = timeout(timing.handshake, socket) => match socket {
            Ok(Ok(socket)) => Ok(socket),
            Ok(Err(_)) => Err(Cause::OperatorClosed.into()),
            Err(_) => Err(Cause::Timeout.into()),
        },
    }
}
