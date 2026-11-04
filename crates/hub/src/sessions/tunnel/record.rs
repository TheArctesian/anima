//! What the hub writes when a redeemed session ends (K4 Section 5, D-9, LOG-005): the row's
//! end, the `session.end` audit entry and the session event, in one transaction, once.
//!
//! Nothing here holds session content, the connection ID, a token or guacd's text (RA-008,
//! LOG-007): the event message is a fixed sentence and the audit change is the reason.

use super::{ending::End, route::Route};
use crate::{
    events::{self, NewEvent, Source},
    ids::SessionId,
    sessions::{Actor, EndReason, Error, PendingTicket, Redeemed, SessionEnd, end::end_row, store},
};

/// Writes the end of `redeemed`'s session, then drops its tunnel registration, so
/// `endSession` reads the ended row once it sees the tunnel gone (D-10). If the write fails
/// it is logged; the row stays live until the hub's next start ends it `HUB_SHUTDOWN`
/// (`close_stale`).
pub(super) async fn finish(route: &Route, redeemed: Redeemed, end: &End) {
    let Redeemed { ticket, tunnel } = redeemed;
    let reason = end.cause.ending().reason;
    match write(route, &ticket, end).await {
        Ok(()) => log_ended(ticket.session, reason),
        Err(error) => log_unwritten(ticket.session, reason, &error),
    }
    drop(tunnel);
}

/// The process log lines for an end carry the session ID and reason only (LOG-007).
fn log_ended(session: SessionId, reason: EndReason) {
    tracing::info!(%session, reason = reason.as_str(), "session ended");
}

fn log_unwritten(session: SessionId, reason: EndReason, error: &Error) {
    tracing::error!(
        %session,
        reason = reason.as_str(),
        %error,
        "the end of a session could not be written"
    );
}

/// The end row and its audit entry and, when this call ended the row, its event. The audit
/// entry names who ended it: the `endSession` operator and address when there is one, else
/// the ticket's operator and client.
async fn write(route: &Route, ticket: &PendingTicket, end: &End) -> Result<(), Error> {
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
        return Ok(());
    }
    let event = NewEvent {
        severity: ending.severity,
        source: Source::Session,
        device: row.and_then(|row| row.device.map(|device| (device, row.device_name))),
        message: end.message.to_owned(),
    };
    events::record(&mut tx, &route.bus, now, event).await?;
    tx.commit().await?;
    Ok(())
}
