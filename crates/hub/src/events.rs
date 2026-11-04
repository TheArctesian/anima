//! The event log and the audit log: what happened and who changed what (LOG-004, LOG-005),
//! and the live broadcast of new events and status changes (LOG-010).

mod bus;
mod model;
mod redaction;
mod store;
#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

pub use bus::{Bus, Signal};
pub use model::{
    AuditAction, AuditTarget, ClientAddr, EventSummary, HeartbeatSummary, NewAudit, NewEvent,
    Operator, Severity, Source,
};

use crate::ids::EventId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The message stays generic: the source error may hold SQL text (API-010).
    #[error("the event store is unavailable")]
    Database(#[from] sqlx::Error),
}

/// Stores the event, then publishes it on `bus` as `Signal::Event`.
///
/// The publish happens inside the caller's transaction: if the caller rolls back, subscribers
/// have seen an event that is not stored (D-5). The subscription is a live view; the stored
/// event list is the truth.
///
/// # Errors
///
/// `Error::Database` if the partition or the row cannot be written.
pub async fn record(
    conn: &mut PgConnection,
    bus: &Bus,
    at: DateTime<Utc>,
    event: NewEvent,
) -> Result<EventId, Error> {
    let summary = store::insert_event(conn, at, event).await?;
    let id = summary.id;
    bus.publish(Signal::Event(summary));
    Ok(id)
}

/// Stores an audit entry. Secret values in `change` are replaced with `"[redacted]"` first
/// (LOG-007); callers should not pass secrets, this is the backstop.
///
/// # Errors
///
/// `Error::Database` if the partition or the row cannot be written.
pub async fn audit(
    conn: &mut PgConnection,
    at: DateTime<Utc>,
    entry: NewAudit,
) -> Result<(), Error> {
    let entry = NewAudit {
        change: redaction::redact(entry.change),
        ..entry
    };
    store::insert_audit(conn, at, &entry).await?;
    Ok(())
}
