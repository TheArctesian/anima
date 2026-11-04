//! Ending sessions: at the operator's request (`endSession`, D-10), by the lazy sweep of
//! tickets that expired unredeemed (D-2), and the row-and-audit write every end shares.
//!
//! An end that never opened a tunnel (expired, refused or withdrawn ticket) writes the
//! `session.end` audit entry and no event: the event log records what a tunnel did (K4 §5).

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgConnection, PgPool};

use super::{
    Error, Missing,
    live::{Ending, Live},
    model::{EndReason, Session, SessionEnd},
    store,
};
use crate::{
    events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator},
    ids::SessionId,
};

/// Who ends a session, from where and when: what its `session.end` audit entry records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    pub operator: Operator,
    pub client: ClientAddr,
    pub at: DateTime<Utc>,
}

/// Ends a live row with `end` and writes the audit entry `session.end` for it, stamped
/// `by.at`. Does nothing when the row has already ended: the first end wins (D-9).
pub(super) async fn end_row(
    conn: &mut PgConnection,
    session: SessionId,
    end: SessionEnd,
    by: &Actor,
) -> Result<(), Error> {
    if !store::end(&mut *conn, session, end).await? {
        return Ok(());
    }
    let audit = NewAudit {
        operator: by.operator.clone(),
        action: AuditAction::End,
        target: AuditTarget::Session(session),
        client: by.client,
        change: json!({ "reason": end.reason.as_str() }),
    };
    events::audit(conn, by.at, audit).await?;
    Ok(())
}

/// Removes every ticket expired at `now`, unredeemed or spent by a refused redemption, and
/// ends its row with `TIMEOUT` at the ticket's expiry, in one transaction. At most
/// `MAX_PENDING` tickets, so the work is bounded. If the rows cannot be ended the tickets go
/// back (still refused) and the next sweep retries.
///
/// # Errors
/// `Database` or `Events` when the rows or audit entries cannot be written.
pub(super) async fn sweep(db: &PgPool, live: &Live, now: DateTime<Utc>) -> Result<(), Error> {
    let expired = live.take_expired(now);
    if expired.is_empty() {
        return Ok(());
    }
    let ended = async {
        let mut tx = db.begin().await?;
        for ticket in &expired {
            let end = SessionEnd {
                at: ticket.expires_at,
                reason: EndReason::Timeout,
            };
            let by = Actor {
                operator: ticket.operator.clone(),
                client: ticket.client,
                at: now,
            };
            end_row(&mut tx, ticket.session, end, &by).await?;
        }
        tx.commit().await.map_err(Error::from)
    }
    .await;
    if ended.is_err() {
        live.restore(expired);
    }
    ended
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
        Ending::Signalled(closing) => {
            // Running out of time is not a failure: the row is returned as it stands either
            // way, live if the tunnel is stuck.
            tokio::time::timeout(live.tunnel_end_wait(), closing.finished())
                .await
                .unwrap_or_default();
        }
        Ending::Withdrawn(ticket) => {
            if let Err(error) = end_by_operator(db, session, by).await {
                live.restore(vec![ticket]);
                return Err(error);
            }
        }
        Ending::Neither => end_by_operator(db, session, by).await?,
    }
    current(db, session).await
}

async fn current(db: &PgPool, session: SessionId) -> Result<Session, Error> {
    store::get(db, session)
        .await?
        .ok_or(Error::NotFound(Missing::Session(session)))
}
