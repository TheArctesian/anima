//! The session task the Guacamole and the terminal tunnel share (K4 Sections 2 and 8.1,
//! MV-003): one task per redeemed ticket that checks the subprotocol, opens the far side while
//! the request waits, unless the session ends first, answers the request with the upgrade,
//! hands over to the tunnel's own part ([`FarSide`]) and ends the session once. Beside it, the
//! sends and the close every WebSocket tunnel makes (`writes`), the silence timer (`quiet`) and
//! what the tunnels serve with (`serving`).
//!
//! The session task, not the request handler, runs all of this, so the client leaving (seen
//! as the handler dropped) ends the session instead of dropping it. It holds the ticket in an
//! `end::Unfinished` guard, so even a panic ends the row.

mod quiet;
mod serving;
mod writes;

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{WebSocketUpgrade, ws::WebSocket},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::{sync::oneshot, time::timeout};

use super::{
    PendingTicket, Redeemed, Tunnel,
    end::{CLIENT_LEFT, Cause, End, Records, Unfinished},
};

pub(super) use quiet::Quiet;
pub use serving::Serving;
pub(super) use writes::{close, send};

/// One tunnel's own part of the session task: how its far side opens for a ticket and what
/// the session does once the request is answered. The terminal (`terminal::session`) and the
/// Guacamole tunnel (`tunnel::relaying`) each have one.
pub(super) trait FarSide {
    /// What a successful opening hands to [`FarSide::connected`].
    type Opening;
    /// The event message when the request did not offer the tunnel's subprotocol.
    const NOT_OFFERED: &'static str;

    /// Where this tunnel's sessions write their end.
    fn records(&self) -> Records;

    /// Opens the far side for `ticket`, safe to drop at any await. A failure is logged here
    /// and is the session's end.
    async fn open(&self, ticket: &PendingTicket) -> Result<Self::Opening, End>;

    /// The session after the response: waits for the upgraded `socket`, then runs the tunnel
    /// or tells the client why there is none.
    async fn connected(
        &self,
        tunnel: &mut Tunnel,
        opening: Self::Opening,
        socket: oneshot::Receiver<WebSocket>,
    ) -> End;
}

/// Runs the session of `redeemed` to its end and writes the end. `upgrade` is the request's
/// upgrade when it offered the tunnel's subprotocol; `answer` takes the handler's response.
pub(super) async fn run_session<F: FarSide>(
    far: Arc<F>,
    redeemed: Redeemed,
    upgrade: Option<WebSocketUpgrade>,
    answer: oneshot::Sender<Response>,
) {
    let mut unfinished = Unfinished::new(far.records(), redeemed);
    let end = session(&*far, unfinished.redeemed(), upgrade, answer).await;
    unfinished.finish(&end).await;
}

/// The session up to its first cause: refusals before the upgrade answer 400 or 503
/// ([`handed_over`]).
async fn session<F: FarSide>(
    far: &F,
    redeemed: &mut Redeemed,
    upgrade: Option<WebSocketUpgrade>,
    answer: oneshot::Sender<Response>,
) -> End {
    let Some(upgrade) = upgrade else {
        let _ = answer.send(StatusCode::BAD_REQUEST.into_response());
        return End::explained(Cause::InvalidInstruction, F::NOT_OFFERED);
    };
    let Redeemed { ticket, tunnel } = redeemed;
    match handed_over(far.open(ticket), upgrade, tunnel, answer).await {
        Ok((opening, socket)) => far.connected(tunnel, opening, socket).await,
        Err(end) => end,
    }
}

/// Runs `opening` unless the session ends first, then answers the request with `upgrade`.
/// The session ends first on `endSession` or shutdown (SEC-002, D-10), or when the client
/// leaves, seen as the request handler dropped; `opening` is then dropped, and with it the
/// connection to the far side, before the credential leaves the hub when it can be. A failed
/// opening answers 503.
///
/// # Errors
/// The session's end when it ended first or the opening failed.
async fn handed_over<T>(
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
