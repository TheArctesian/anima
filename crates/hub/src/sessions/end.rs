//! Ending sessions: at the operator's request (`endSession`, D-10), by the lazy sweep of
//! tickets that expired unredeemed (D-2), at hub shutdown and at the next start for rows a
//! previous process left open, and the row-and-audit write every end shares.
//!
//! An end that never opened a tunnel (expired, refused, withdrawn or shut-down ticket, or a
//! stale row whose tunnel went with the old process) writes the `session.end` audit entry and
//! no event: the event log records what a tunnel did (K4 §5, D-13).
//!
//! An open tunnel ends through the end table (`cause`) and writes its end once (`record`).

mod cause;
mod record;

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgConnection, PgPool};

use super::{
    Error, Missing,
    live::{self, Live},
    model::{EndReason, Session, SessionEnd},
    store,
    tickets::PendingTicket,
};
use crate::{
    events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator},
    ids::SessionId,
};

#[cfg(test)]
pub(super) use cause::Ending;
pub(super) use cause::{CLIENT_IDLE, CLOSE_WAIT, Cause, End};
#[cfg(test)]
pub(super) use record::finish;
pub(super) use record::{Records, Unfinished};

/// Who ends a session, from where and when: what its `session.end` audit entry records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    pub operator: Operator,
    pub client: ClientAddr,
    pub at: DateTime<Utc>,
}

/// Ends a live row with `end` and writes the audit entry `session.end` for it, stamped
/// `by.at`. Does nothing when the row has already ended: the first end wins (D-9). Returns
/// whether this call ended it.
pub(super) async fn end_row(
    conn: &mut PgConnection,
    session: SessionId,
    end: SessionEnd,
    by: &Actor,
) -> Result<bool, Error> {
    if !store::end(&mut *conn, session, end).await? {
        return Ok(false);
    }
    audit_end(conn, session, end.reason, by).await?;
    Ok(true)
}

/// The audit entry `session.end` for a row this caller ended: who ended it, from where, at
/// `by.at`, and the reason, never more (LOG-007).
async fn audit_end(
    conn: &mut PgConnection,
    session: SessionId,
    reason: EndReason,
    by: &Actor,
) -> Result<(), Error> {
    let audit = NewAudit {
        operator: by.operator.clone(),
        action: AuditAction::End,
        target: AuditTarget::Session(session),
        client: by.client,
        change: json!({ "reason": reason.as_str() }),
    };
    events::audit(conn, by.at, audit).await?;
    Ok(())
}

/// Ends each ticket's row with its end and the audit entry naming the ticket's operator and
/// client, stamped `now`, in one transaction. If the rows cannot be ended the tickets go back
/// (refused, without a token) and the sweep retries.
async fn end_tickets(
    db: &PgPool,
    live: &Live,
    ends: Vec<(PendingTicket, SessionEnd)>,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    if ends.is_empty() {
        return Ok(());
    }
    let ended = async {
        let mut tx = db.begin().await?;
        for (ticket, end) in &ends {
            let by = Actor {
                operator: ticket.operator.clone(),
                client: ticket.client,
                at: now,
            };
            end_row(&mut tx, ticket.session, *end, &by).await?;
        }
        tx.commit().await.map_err(Error::from)
    }
    .await;
    if ended.is_err() {
        live.restore(ends.into_iter().map(|(ticket, _)| ticket).collect());
    }
    ended
}

/// Removes every ticket expired at `now`, unredeemed or spent by a refused redemption, and
/// ends its row with `TIMEOUT` at the ticket's expiry, in one transaction. At most
/// `MAX_PENDING` tickets, so the work is bounded. If the rows cannot be ended the tickets go
/// back (still refused) and the next sweep retries.
///
/// # Errors
/// `Database` or `Events` when the rows or audit entries cannot be written.
pub(super) async fn sweep(db: &PgPool, live: &Live, now: DateTime<Utc>) -> Result<(), Error> {
    let expired = live.take_expired(now).into_iter().map(|ticket| {
        let end = SessionEnd {
            at: ticket.expires_at,
            reason: EndReason::Timeout,
        };
        (ticket, end)
    });
    end_tickets(db, live, expired.collect(), now).await
}

/// Ends every session `live` holds with `HUB_SHUTDOWN` at `now`, and from then on refuses
/// new tickets and redemptions (D-10, K4 §5):
///
/// - each open tunnel is told to end; it closes with 1001 and writes its own row end, audit
///   entry and event. This waits up to `live`'s tunnel end wait for them to finish, and
///   returns when they have or the wait runs out;
/// - each unredeemed or spent ticket (never a tunnel) is withdrawn and its row ends here, at
///   `now`, with the `session.end` audit entry naming the ticket's operator and client, and no
///   event (D-13).
///
/// A row this misses (a tunnel past the wait, a start whose commit lands after this, a ticket
/// put back by a failed write) stays live until the next start's [`close_stale`].
///
/// # Errors
/// `Database` or `Events` when the ticket rows or their audit entries cannot be written; the
/// tunnels have still been told to end and waited for.
pub(super) async fn shut_down(db: &PgPool, live: &Live, now: DateTime<Utc>) -> Result<(), Error> {
    let (closing, tickets) = live.shut_down();
    let end = SessionEnd {
        at: now,
        reason: EndReason::HubShutdown,
    };
    let ends = tickets.into_iter().map(|ticket| (ticket, end)).collect();
    let ended = end_tickets(db, live, ends, now).await;
    // Running out of time is not a failure: the next start's `close_stale` ends what is left.
    tokio::time::timeout(live.tunnel_end_wait(), closing.finished())
        .await
        .unwrap_or_default();
    ended
}

/// Ends, with `HUB_SHUTDOWN` at `now`, every row a previous hub process left live, and writes
/// one `session.end` audit entry for each, naming the row's operator and client, in one
/// transaction. Audit only, no event (D-13): the row's tunnel went with the old process, so
/// no tunnel ends here. Returns how many rows it ended.
///
/// # Errors
/// `Database` or `Events` when the rows or audit entries cannot be written; then none is.
pub(super) async fn close_stale(db: &PgPool, now: DateTime<Utc>) -> Result<u64, Error> {
    let mut tx = db.begin().await?;
    let stale = store::close_stale(&mut *tx, now).await?;
    for row in &stale {
        let by = Actor {
            operator: row.operator.clone(),
            client: row.client,
            at: now,
        };
        audit_end(&mut tx, row.session, EndReason::HubShutdown, &by).await?;
    }
    tx.commit().await?;
    Ok(u64::try_from(stale.len()).unwrap_or(u64::MAX))
}

/// Ends the row with `OPERATOR` and its audit entry, in one transaction.
async fn end_by_operator(db: &PgPool, session: SessionId, by: &Actor) -> Result<(), Error> {
    let end = SessionEnd {
        at: by.at,
        reason: EndReason::Operator,
    };
    let mut tx = db.begin().await?;
    end_row(&mut tx, session, end, by).await?;
    tx.commit().await?;
    Ok(())
}

/// Ends the session with `OPERATOR` and returns its row.
///
/// - An open tunnel is told to end, with `by`, and given up to `live`'s tunnel end wait to
///   write its end; the row is returned as it then stands (still live if the tunnel did not
///   finish in time). The tunnel writes the audit entry.
/// - Otherwise a pending or spent ticket is withdrawn and a live row ends here, with its
///   audit entry. If that write fails, the ticket goes back so the sweep still ends the row.
/// - An ended session is returned unchanged.
///
/// # Errors
/// `NotFound` when there is no such session; `Database` or `Events` when a write fails.
pub(super) async fn end_session(
    db: &PgPool,
    live: &Live,
    session: SessionId,
    by: &Actor,
) -> Result<Session, Error> {
    match live.end_by(session, by) {
        live::Ending::Signalled(closing) => {
            // Running out of time is not a failure: the row is returned as it stands either
            // way, live if the tunnel is stuck.
            tokio::time::timeout(live.tunnel_end_wait(), closing.finished())
                .await
                .unwrap_or_default();
        }
        live::Ending::Withdrawn(ticket) => {
            if let Err(error) = end_by_operator(db, session, by).await {
                live.restore(vec![ticket]);
                return Err(error);
            }
        }
        live::Ending::Neither => end_by_operator(db, session, by).await?,
    }
    current(db, session).await
}

async fn current(db: &PgPool, session: SessionId) -> Result<Session, Error> {
    store::get(db, session)
        .await?
        .ok_or(Error::NotFound(Missing::Session(session)))
}
