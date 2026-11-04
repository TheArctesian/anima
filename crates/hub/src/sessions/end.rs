//! Ending a session at the operator's request (`endSession`, D-10).

use std::time::Duration;

use sqlx::PgPool;

use super::{
    Error, Missing,
    live::Live,
    model::{EndReason, Session, SessionEnd},
    start::{Actor, end_row},
    store,
};
use crate::ids::SessionId;

/// How long `endSession` waits for an open tunnel to end its row (D-10).
pub(super) const TUNNEL_END_WAIT: Duration = Duration::from_secs(5);

/// Ends the session with `OPERATOR` and returns its row.
///
/// - An open tunnel is told to end and given up to [`TUNNEL_END_WAIT`] to write its end; the
///   row is returned as it then stands (still live if the tunnel did not finish in time).
/// - Otherwise a pending ticket is withdrawn and a live row ends here, with its audit entry.
/// - An ended session is returned unchanged.
///
/// # Errors
/// `NotFound` when there is no such session; `Database` or `Events` when a write fails.
pub(super) async fn end_session(
    db: &PgPool,
    live: &Live,
    session: SessionId,
    by: &Actor<'_>,
) -> Result<Session, Error> {
    if let Some(closing) = live.end_live(session) {
        // Running out of time is not a failure: the row is returned as it stands either way,
        // live if the tunnel is stuck.
        tokio::time::timeout(TUNNEL_END_WAIT, closing.finished())
            .await
            .unwrap_or_default();
        return current(db, session).await;
    }
    live.take_pending(session);
    let end = SessionEnd {
        at: by.at,
        reason: EndReason::Operator,
    };
    let mut tx = db.begin().await?;
    end_row(&mut tx, session, end, by).await?;
    tx.commit().await?;
    current(db, session).await
}

async fn current(db: &PgPool, session: SessionId) -> Result<Session, Error> {
    store::get(db, session)
        .await?
        .ok_or(Error::NotFound(Missing::Session(session)))
}
