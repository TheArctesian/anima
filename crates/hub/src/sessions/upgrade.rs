//! The WebSocket upgrade of a redeemed ticket's request, shared by the Guacamole and the
//! terminal tunnel (K4 Sections 2 and 8.1, MV-003): the far side opens while the request
//! waits, unless the session ends first; the request is then answered with the upgrade, and
//! the session waits for the upgraded socket.
//!
//! The session task, not the request handler, runs all of this, so the client leaving (seen
//! as the handler dropped) ends the session instead of dropping it.

use std::time::Duration;

use axum::{
    extract::{WebSocketUpgrade, ws::WebSocket},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::{sync::oneshot, time::timeout};

use super::{
    Tunnel,
    end::{CLIENT_LEFT, Cause, End},
};

/// Runs `opening` unless the session ends first, then answers the request with `upgrade`.
/// The session ends first on `endSession` or shutdown (SEC-002, D-10), or when the client
/// leaves, seen as the request handler dropped; `opening` is then dropped, and with it the
/// connection to the far side, before the credential leaves the hub when it can be. A failed
/// opening answers 503.
///
/// # Errors
/// The session's end when it ended first or the opening failed.
pub(super) async fn handed_over<T>(
    opening: impl Future<Output = Result<T, End>>,
    upgrade: WebSocketUpgrade,
    tunnel: &mut Tunnel,
    mut answer: oneshot::Sender<Response>,
) -> Result<(T, oneshot::Receiver<WebSocket>), End> {
    let opened = tokio::select! {
        opened = opening => opened,
        signal = tunnel.ended() => Err(End::from(signal)),
        () = answer.closed() => Err(End::worded(Cause::OperatorClosed, CLIENT_LEFT)),
    };
    let opened = match opened {
        Ok(opened) => opened,
        Err(end) => {
            // Unanswered only when the client has left.
            let _ = answer.send(StatusCode::SERVICE_UNAVAILABLE.into_response());
            return Err(end);
        }
    };
    let (socket_tx, socket) = oneshot::channel::<WebSocket>();
    let response = upgrade.on_upgrade(move |socket| async move {
        // Unsent only when the session has already ended; the socket closes as it drops.
        let _ = socket_tx.send(socket);
    });
    // Unanswered when the client has left: the upgrade then never comes, and `upgraded`
    // ends the session.
    let _ = answer.send(response);
    Ok((opened, socket))
}

/// The upgraded socket, unless the session ends first: the end signal, a failed upgrade (the
/// client left, `OPERATOR`) or no upgrade within `wait` (`TIMEOUT`).
///
/// # Errors
/// The session's end when there is no socket.
pub(super) async fn upgraded(
    tunnel: &mut Tunnel,
    socket: oneshot::Receiver<WebSocket>,
    wait: Duration,
) -> Result<WebSocket, End> {
    tokio::select! {
        signal = tunnel.ended() => Err(End::from(signal)),
        socket = timeout(wait, socket) => match socket {
            Ok(Ok(socket)) => Ok(socket),
            Ok(Err(_)) => Err(Cause::OperatorClosed.into()),
            Err(_) => Err(Cause::Timeout.into()),
        },
    }
}
