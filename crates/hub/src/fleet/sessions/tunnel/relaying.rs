//! The Guacamole tunnel's part of the session task (K4 Sections 2 to 5, `upgrade::FarSide`):
//! it opens the gateway side, and once the `/tunnel` request is answered waits for the
//! upgraded socket, runs the tunnel (`link`) or tells the client the certificate changed. The
//! shared task (`upgrade::run_session`) owns the ticket from redeem to end.
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

use axum::extract::ws::WebSocket;
use tokio::sync::oneshot;

use super::{
    ending::{Cause, End},
    gateway::certificate_changed,
    opening::{self, Opening},
    route::{Route, Timing},
};
use crate::fleet::sessions::{
    PendingTicket, Tunnel,
    upgrade::{FarSide, close, upgraded},
};
use writes::{leave_gateway, send_text};

/// The event message when the request did not offer the `guacamole` subprotocol.
const NOT_GUACAMOLE: &str = "The client did not open the tunnel as a Guacamole WebSocket, so \
                             the hub closed the session.";

impl FarSide for Route {
    type Opening = Opening;
    const NOT_OFFERED: &'static str = NOT_GUACAMOLE;

    async fn open(&self, ticket: &PendingTicket) -> Result<Opening, End> {
        let (session, protocol) = (ticket.session, ticket.protocol);
        opening::open(self, ticket).await.map_err(|not_opened| {
            tracing::warn!(%session, error = %not_opened, "the tunnel did not open");
            End::explained(Cause::GatewayError, not_opened.message(protocol))
        })
    }

    async fn connected(
        &self,
        tunnel: &mut Tunnel,
        opening: Opening,
        socket: oneshot::Receiver<WebSocket>,
    ) -> End {
        connected(self.timing, tunnel, opening, socket).await
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
    close(client, None, cause).await;
    cause.into()
}
