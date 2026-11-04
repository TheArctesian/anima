//! What the hub writes when a redeemed session ends (K4 Section 5, D-9, LOG-005): the row's
//! end, the `session.end` audit entry and the session event, in one transaction, once.
//!
//! Nothing here holds session content, the connection ID, a token or guacd's text (RA-008,
//! LOG-007): the event message is a fixed sentence and the audit change is the reason.

use std::sync::Arc;

use tokio::runtime::Handle;

use super::{
    ending::{Cause, End},
    route::Route,
};
use crate::{
    events::{self, NewEvent, Source},
    ids::SessionId,
    sessions::{Actor, EndReason, Error, PendingTicket, Redeemed, SessionEnd, end::end_row, store},
};

/// The event message when the tunnel task stopped without an end of its own.
const TASK_STOPPED: &str = "The hub's tunnel stopped unexpectedly and closed the SSH session.";

/// A redeemed session whose end is not written yet. The session task holds it from the
/// moment it owns the ticket; [`Unfinished::finish`] writes the end. If the task stops
/// before that (a panic), dropping this writes a `GATEWAY_ERROR` end from a new task, so the
/// row never stays live (MV-003, LOG-005).
pub(super) struct Unfinished {
    route: Arc<Route>,
    redeemed: Option<Redeemed>,
}

impl Unfinished {
    pub(super) const fn new(route: Arc<Route>, redeemed: Redeemed) -> Self {
        Self {
            route,
            redeemed: Some(redeemed),
        }
    }

    /// The session, until it is finished.
    pub(super) fn redeemed(&mut self) -> &mut Redeemed {
        self.redeemed
            .as_mut()
            .expect("only `finish` takes the session, and it consumes the guard")
    }

    /// Writes `end` (see [`finish`]).
    pub(super) async fn finish(mut self, end: &End) {
        if let Some(redeemed) = self.redeemed.take() {
            finish(&self.route, redeemed, end).await;
        }
    }
}

impl Drop for Unfinished {
    fn drop(&mut self) {
        let Some(redeemed) = self.redeemed.take() else {
            return;
        };
        let session = redeemed.ticket.session;
        tracing::error!(%session, "the tunnel task stopped before the session ended");
        let route = Arc::clone(&self.route);
        let end = End::explained(Cause::GatewayError, TASK_STOPPED);
        // With no runtime left the hub is stopping; the next start ends the row
        // `HUB_SHUTDOWN` (`close_stale`).
        if let Ok(runtime) = Handle::try_current() {
            runtime.spawn(async move { finish(&route, redeemed, &end).await });
        }
    }
}

/// Writes the end of `redeemed`'s session, then drops its tunnel registration, so
/// `endSession` reads the ended row once it sees the tunnel gone (D-10). A failed write is
/// tried once more, then logged; the row then stays live until the hub's next start ends it
/// `HUB_SHUTDOWN` (`close_stale`).
pub(super) async fn finish(route: &Route, redeemed: Redeemed, end: &End) {
    let Redeemed { ticket, tunnel } = redeemed;
    let reason = end.cause.ending().reason;
    let written = match write(route, &ticket, end).await {
        Err(_) => write(route, &ticket, end).await,
        first => first,
    };
    match written {
        Ok(true) => log_ended(ticket.session, reason),
        Ok(false) => log_ended_before(ticket.session, reason),
        Err(error) => log_unwritten(ticket.session, reason, &error),
    }
    drop(tunnel);
}

/// The process log lines for an end carry the session ID and reason only (LOG-007).
fn log_ended(session: SessionId, reason: EndReason) {
    tracing::info!(%session, reason = reason.as_str(), "session ended");
}

fn log_ended_before(session: SessionId, reason: EndReason) {
    tracing::info!(
        %session,
        reason = reason.as_str(),
        "the tunnel closed a session that had already ended"
    );
}

fn log_unwritten(session: SessionId, reason: EndReason, error: &Error) {
    tracing::error!(
        %session,
        reason = reason.as_str(),
        %error,
        "the end of a session could not be written"
    );
}

/// The end row and its audit entry and, when this call ended the row, its event; `false`
/// when the row had already ended. The audit entry names who ended it: the `endSession`
/// operator and address when there is one, else the ticket's operator and client.
async fn write(route: &Route, ticket: &PendingTicket, end: &End) -> Result<bool, Error> {
    let ending = end.cause.ending();
    let now = route.clock.now();
    let by = end.by.clone().unwrap_or_else(|| Actor {
        operator: ticket.operator.clone(),
        client: ticket.client,
        at: now,
    });
    let ended = SessionEnd {
        at: now,
        reason: ending.reason,
    };
    let mut tx = route.db.begin().await?;
    let row = store::get(&mut *tx, ticket.session).await?;
    if !end_row(&mut tx, ticket.session, ended, &by).await? {
        return Ok(false);
    }
    let event = NewEvent {
        severity: ending.severity,
        source: Source::Session,
        device: row.and_then(|row| row.device.map(|device| (device, row.device_name))),
        message: end.message.to_owned(),
    };
    events::record(&mut tx, &route.bus, now, event).await?;
    tx.commit().await?;
    Ok(true)
}
