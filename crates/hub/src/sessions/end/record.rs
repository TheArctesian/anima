//! What the hub writes when a redeemed session ends (K4 Section 5, D-9, LOG-005): the row's
//! end, the `session.end` audit entry and the session event, in one transaction, once.
//!
//! Both tunnels write their end here. Nothing here holds session content, the connection ID,
//! a token or the far side's text (RA-008, LOG-007): the event message is a fixed sentence and
//! the audit change is the reason.

use std::sync::Arc;

use sqlx::PgPool;
use tokio::runtime::Handle;

use super::{
    cause::{Cause, End, TASK_STOPPED},
    end_row,
};
use crate::{
    clock::Clock,
    events::{self, Bus, NewEvent, Source},
    ids::SessionId,
    sessions::{Actor, EndReason, Error, PendingTicket, Redeemed, Session, SessionEnd},
};

/// Where an end is written: the database, the bus its event goes out on, and the clock that
/// stamps it.
#[derive(Clone)]
pub(in crate::sessions) struct Records {
    pub(in crate::sessions) db: PgPool,
    pub(in crate::sessions) bus: Bus,
    pub(in crate::sessions) clock: Arc<dyn Clock>,
}

impl Records {
    /// The records of a route that holds the three parts itself.
    pub(in crate::sessions) fn new(db: &PgPool, bus: &Bus, clock: &Arc<dyn Clock>) -> Self {
        Self {
            db: db.clone(),
            bus: bus.clone(),
            clock: Arc::clone(clock),
        }
    }
}

/// A redeemed session whose end is not written yet. The session task holds it from the
/// moment it owns the ticket; [`Unfinished::finish`] writes the end. If the task stops
/// before that (a panic), dropping this writes the end from a new task (MV-003, LOG-005):
/// the cause `endSession` or shutdown had already signalled, with who asked, else
/// `GATEWAY_ERROR` (D-9).
///
/// A runtime that is shutting down drops that new task unrun, and so does a drop with no
/// runtime at all; the row then stays live until the hub's next start ends it
/// `HUB_SHUTDOWN` (`close_stale`, handoff H-2).
pub(in crate::sessions) struct Unfinished {
    records: Records,
    redeemed: Option<Redeemed>,
}

impl Unfinished {
    pub(in crate::sessions) const fn new(records: Records, redeemed: Redeemed) -> Self {
        Self {
            records,
            redeemed: Some(redeemed),
        }
    }

    /// The session, until it is finished.
    pub(in crate::sessions) fn redeemed(&mut self) -> &mut Redeemed {
        self.redeemed
            .as_mut()
            .expect("only `finish` takes the session, and it consumes the guard")
    }

    /// Writes `end` (see [`finish`]).
    pub(in crate::sessions) async fn finish(mut self, end: &End) {
        if let Some(redeemed) = self.redeemed.take() {
            finish(&self.records, redeemed, end).await;
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
        let records = self.records.clone();
        let end = redeemed
            .tunnel
            .signalled()
            .map_or_else(|| End::worded(Cause::GatewayError, TASK_STOPPED), End::from);
        if let Ok(runtime) = Handle::try_current() {
            runtime.spawn(async move { finish(&records, redeemed, &end).await });
        }
    }
}

/// Writes the end of `redeemed`'s session and publishes the ended row, then drops its tunnel
/// registration, so `endSession` reads the ended row once it sees the tunnel gone. A
/// failed write is tried once more, then logged; the row then stays live until the hub's next
/// start ends it `HUB_SHUTDOWN` (`close_stale`).
pub(in crate::sessions) async fn finish(records: &Records, redeemed: Redeemed, end: &End) {
    let Redeemed { ticket, tunnel } = redeemed;
    let reason = end.cause.ending().reason;
    let written = match write(records, &ticket, end).await {
        Err(_) => write(records, &ticket, end).await,
        first => first,
    };
    match written {
        Ok(Some(ended)) => {
            log_ended(ticket.session, reason);
            tunnel.live().publish_change(ended);
        }
        Ok(None) => log_ended_before(ticket.session, reason),
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

/// The end row and its audit entry and, when this call ended the row, its event; returns the
/// committed row, or `None` when it had already ended. The audit entry names who ended it:
/// the `endSession` operator and address when there is one, else the ticket's operator and
/// client.
async fn write(
    records: &Records,
    ticket: &PendingTicket,
    end: &End,
) -> Result<Option<Session>, Error> {
    let ending = end.cause.ending();
    let now = records.clock.now();
    let by = end.by.clone().unwrap_or_else(|| Actor {
        operator: ticket.operator.clone(),
        client: ticket.client,
        at: now,
    });
    let ended = SessionEnd {
        at: now,
        reason: ending.reason,
    };
    let mut tx = records.db.begin().await?;
    let Some(row) = end_row(&mut tx, ticket.session, ended, &by).await? else {
        return Ok(None);
    };
    let event = NewEvent {
        severity: ending.severity,
        source: Source::Session,
        device: row.device.map(|device| (device, row.device_name.clone())),
        message: end.message.of(ticket.protocol).to_owned(),
        actor: by.operator.actor.kind(),
    };
    events::record(&mut tx, &records.bus, now, event).await?;
    tx.commit().await?;
    Ok(Some(row))
}
