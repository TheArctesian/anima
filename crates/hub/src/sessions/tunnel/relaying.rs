//! The session task (K4 Sections 2 to 5): one task per redeemed ticket that opens the gateway
//! side, answers the `/tunnel` request and waits for the upgraded socket (`upgrade`), runs the
//! tunnel (`link`) and ends the session once.
//!
//! The task, not the request handler, owns the ticket: hyper drops the handler when the
//! client goes, and the session must still end (MV-003). It holds the ticket in an
//! [`Unfinished`] guard, so even a panic ends the row.
//!
//! A known gap, accepted (SEC-002, issue #1): the opening stops when the client leaves only
//! if hyper notices, and hyper notices an EOF during a request only while its read buffer is
//! empty. A client that sends even one byte after the request headers and then leaves is not
//! seen: the opening carries on, the credential still reaches guacd, and the session ends
//! when the upgrade never comes (or the opening fails) instead of at once. Only the holder of
//! a valid ticket, from the address it is bound to, can do this, and the session still ends
//! once.

mod link;
mod writes;

use std::sync::Arc;

use axum::{
    extract::{WebSocketUpgrade, ws::WebSocket},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::sync::oneshot;

use super::{
    ending::{Cause, End},
    gateway::certificate_changed,
    opening::{self, Opening},
    route::{Route, Timing},
};
use crate::sessions::{
    Redeemed, Tunnel,
    end::Unfinished,
    upgrade::{handed_over, upgraded},
};
use writes::{close_client, leave_gateway, send_text};

/// The event message when the request did not offer the `guacamole` subprotocol.
const NOT_GUACAMOLE: &str = "The client did not open the tunnel as a Guacamole WebSocket, so \
                             the hub closed the session.";
/// Runs the session of `redeemed` to its end and writes the end. `upgrade` is the request's
/// upgrade when it offered `guacamole`; `answer` takes the handler's response.
pub(super) async fn run(
    route: Arc<Route>,
    redeemed: Redeemed,
    upgrade: Option<WebSocketUpgrade>,
    answer: oneshot::Sender<Response>,
) {
    let mut unfinished = Unfinished::new(route.records(), redeemed);
    let end = session(&route, unfinished.redeemed(), upgrade, answer).await;
    unfinished.finish(&end).await;
}

/// The session up to its first cause: refusals before the upgrade answer 400 or 503
/// (`upgrade`).
async fn session(
    route: &Route,
    redeemed: &mut Redeemed,
    upgrade: Option<WebSocketUpgrade>,
    answer: oneshot::Sender<Response>,
) -> End {
    let Some(upgrade) = upgrade else {
        let _ = answer.send(StatusCode::BAD_REQUEST.into_response());
        return End::explained(Cause::InvalidInstruction, NOT_GUACAMOLE);
    };
    let Redeemed { ticket, tunnel } = redeemed;
    let (session, protocol) = (ticket.session, ticket.protocol);
    let opening = async {
        opening::open(route, ticket).await.map_err(|not_opened| {
            tracing::warn!(%session, error = %not_opened, "the tunnel did not open");
            End::explained(Cause::GatewayError, not_opened.message(protocol))
        })
    };
    match handed_over(opening, upgrade, tunnel, answer).await {
        Ok((opening, socket)) => connected(route.timing, tunnel, opening, socket).await,
        Err(end) => end,
    }
}

/// The tunnel after the response: waits for the upgraded socket, then relays, or tells the
/// client the certificate changed. When the upgrade fails, guacd is told the hub is leaving;
/// after a changed certificate the session still ends `SERVER_CERTIFICATE_MISMATCH` (SEC-007).
pub(super) async fn connected(
    timing: Timing,
    tunnel: &mut Tunnel,
    opening: Opening,
    socket: oneshot::Receiver<WebSocket>,
) -> End {
    let client = upgraded(tunnel, socket, timing.handshake).await;
    match (opening, client) {
        (Opening::Gateway(opened), Ok(client)) => {
            link::relay(client, *opened, tunnel, timing).await
        }
        (Opening::Gateway(opened), Err(end)) => {
            leave_gateway(opened.connected.stream).await;
            end
        }
        (Opening::CertificateChanged, Ok(client)) => {
            certificate_mismatch(client, tunnel, timing).await
        }
        (Opening::CertificateChanged, Err(_)) => Cause::ServerCertificateMismatch.into(),
    }
}

/// Tells the client the certificate changed and closes 4003; guacd was never reached (K4
/// Section 5). The mismatch is the cause even if the client is gone before it hears of it.
async fn certificate_mismatch(mut client: WebSocket, tunnel: &mut Tunnel, timing: Timing) -> End {
    let cause = Cause::ServerCertificateMismatch;
    let _ = send_text(
        &mut client,
        &certificate_changed(),
        tunnel,
        timing.client_idle,
    )
    .await;
    close_client(client, cause).await;
    cause.into()
}
