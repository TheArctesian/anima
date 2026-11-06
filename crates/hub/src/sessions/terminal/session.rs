//! The session task (K4 Section 8): one task per redeemed ticket that opens the device side,
//! answers the `/terminal` request, waits for the upgraded socket, runs the shell (`shell`)
//! and ends the session once.
//!
//! The task, not the request handler, owns the ticket: hyper drops the handler when the
//! client goes, and the session must still end (MV-003). It holds the ticket in an
//! `end::Unfinished` guard, so even a panic ends the row.
//!
//! The known gap of the Guacamole tunnel holds here too (SEC-002, issue #1): a client that
//! sends a byte after the request headers and then leaves is not seen until the upgrade
//! never comes, so the opening, the sign-in included, carries on for that session.

use std::sync::Arc;

use axum::{
    extract::{WebSocketUpgrade, ws::WebSocket},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::{sync::oneshot, time::timeout};

use super::{
    ending::{CLIENT_LEFT, NOT_TERMINAL, ended},
    opening::{self, Opening},
    route::{Terminal, Timing},
    shell,
    writes::{close_client, leave_unstarted},
};
use crate::sessions::{
    Redeemed, Tunnel,
    end::{Cause, End, Unfinished},
};

/// Runs the session of `redeemed` to its end and writes the end. `upgrade` is the request's
/// upgrade when it offered `anima-terminal`; `answer` takes the handler's response.
pub(super) async fn run(
    terminal: Arc<Terminal>,
    redeemed: Redeemed,
    upgrade: Option<WebSocketUpgrade>,
    answer: oneshot::Sender<Response>,
) {
    let mut unfinished = Unfinished::new(terminal.records(), redeemed);
    let end = session(&terminal, unfinished.redeemed(), upgrade, answer).await;
    unfinished.finish(&end).await;
}

/// The session up to its first cause: refusals before the upgrade answer 400 or 503.
async fn session(
    terminal: &Terminal,
    redeemed: &mut Redeemed,
    upgrade: Option<WebSocketUpgrade>,
    mut answer: oneshot::Sender<Response>,
) -> End {
    let Some(upgrade) = upgrade else {
        let _ = answer.send(StatusCode::BAD_REQUEST.into_response());
        return End::explained(Cause::InvalidInstruction, NOT_TERMINAL);
    };
    let opening = match opened(terminal, redeemed, &mut answer).await {
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
    connected(terminal.timing, &mut redeemed.tunnel, opening, socket_rx).await
}

/// The opening, unless the session ends first: `endSession` or shutdown (SEC-002, D-10;
/// the opening is dropped, and with it the connection, before the credential leaves the hub
/// when it can be), or the client leaving, seen as the request handler dropped.
async fn opened(
    terminal: &Terminal,
    redeemed: &mut Redeemed,
    answer: &mut oneshot::Sender<Response>,
) -> Result<Opening, End> {
    let Redeemed { ticket, tunnel } = redeemed;
    let session = ticket.session;
    tokio::select! {
        opened = opening::open(terminal, ticket) => opened.map_err(|not_opened| {
            tracing::warn!(%session, error = %not_opened, "the terminal did not open");
            End::explained(Cause::GatewayError, not_opened.message())
        }),
        signal = tunnel.ended() => Err(End::from(signal)),
        () = answer.closed() => Err(End::explained(Cause::OperatorClosed, CLIENT_LEFT)),
    }
}

/// The session after the response: waits for the upgraded socket, then runs the shell or
/// tells the client why there is none.
pub(super) async fn connected(
    timing: Timing,
    tunnel: &mut Tunnel,
    opening: Opening,
    socket: oneshot::Receiver<WebSocket>,
) -> End {
    match opening {
        Opening::Shell(shell) => match upgraded(tunnel, socket, timing).await {
            Ok(client) => shell::run(client, shell, tunnel, timing).await,
            Err(end) => {
                leave_unstarted(shell).await;
                end
            }
        },
        Opening::HostKeyChanged => {
            let end = End::from(Cause::HostKeyMismatch);
            refused(end, tunnel, socket, timing).await
        }
        Opening::SignInRefused(message) => {
            let end = End::explained(Cause::AuthFailed, message);
            refused(end, tunnel, socket, timing).await
        }
    }
}

/// Tells the client, once upgraded, why there is no shell, and closes with the end's code.
/// The end is the cause even if the upgrade fails: it came first (K4 Section 8.3).
async fn refused(
    end: End,
    tunnel: &mut Tunnel,
    socket: oneshot::Receiver<WebSocket>,
    timing: Timing,
) -> End {
    if let Ok(client) = upgraded(tunnel, socket, timing).await {
        close_client(client, end.cause).await;
    }
    end
}

/// The upgraded socket, unless the session ends first: the end signal, a failed upgrade (the
/// client left) or no upgrade within the upgrade wait.
async fn upgraded(
    tunnel: &mut Tunnel,
    socket: oneshot::Receiver<WebSocket>,
    timing: Timing,
) -> Result<WebSocket, End> {
    tokio::select! {
        signal = tunnel.ended() => Err(End::from(signal)),
        socket = timeout(timing.upgrade, socket) => match socket {
            Ok(Ok(socket)) => Ok(socket),
            Ok(Err(_)) => Err(ended(Cause::OperatorClosed)),
            Err(_) => Err(ended(Cause::Timeout)),
        },
    }
}
