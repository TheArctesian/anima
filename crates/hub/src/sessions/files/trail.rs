//! What each transfer leaves behind (K4 Section 9, LOG-005, LOG-007): one audit entry
//! `file.upload` or `file.download` by the ticket's operator and client address, with the path,
//! the size, the bytes moved and the outcome, never content; and one event with a fixed
//! sentence. The path stays in the audit entry: the event log is read more widely.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;

use super::{
    protocol::Refusal,
    tickets::FilesTicket,
    transfer::{Direction, Outcome, Transfer},
};
use crate::{
    clock::Clock,
    devices::Device,
    events::{
        self, ActorKind, AuditAction, AuditTarget, Bus, NewEvent, Severity, Source, TrailEntry,
    },
    sessions::Error,
};

/// Where a files tunnel writes: the stores, the clock, who opened it and the device's name.
pub(super) struct Trail {
    pub(super) db: PgPool,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) ticket: FilesTicket,
    pub(super) device_name: String,
}

/// The event's sentence and severity: fixed text with no path, value or device text.
pub(super) const fn sentence(direction: Direction, outcome: Outcome) -> (Severity, &'static str) {
    use Direction::{Download, Upload};
    match (direction, outcome) {
        (Download, Outcome::Done) => (
            Severity::Info,
            "An operator downloaded a file from the device.",
        ),
        (Upload, Outcome::Done) => (Severity::Info, "An operator uploaded a file to the device."),
        (Download, Outcome::Refused) => (
            Severity::Info,
            "The hub refused a file download from the device.",
        ),
        (Upload, Outcome::Refused) => (
            Severity::Info,
            "The hub refused a file upload to the device.",
        ),
        (Download, Outcome::Cancelled) => (
            Severity::Info,
            "A file download from the device was cancelled.",
        ),
        (Upload, Outcome::Cancelled) => {
            (Severity::Info, "A file upload to the device was cancelled.")
        }
        (Download, Outcome::Failed) => {
            (Severity::Warning, "A file download from the device failed.")
        }
        (Upload, Outcome::Failed) => (Severity::Warning, "A file upload to the device failed."),
    }
}

impl Trail {
    /// Writes the audit entry and the event of `transfer`, ended with `outcome` (and
    /// `reason` when refused), in one transaction.
    ///
    /// # Errors
    /// `Database` or `Events` when either cannot be written; then neither is.
    pub(super) async fn record(
        &self,
        transfer: &Transfer,
        outcome: Outcome,
        reason: Option<Refusal>,
    ) -> Result<(), Error> {
        let action = match transfer.direction {
            Direction::Upload => AuditAction::Upload,
            Direction::Download => AuditAction::Download,
        };
        let ticket = &self.ticket;
        let (severity, message) = sentence(transfer.direction, outcome);
        let entry = TrailEntry {
            source: Source::Session,
            severity,
            device: Some((ticket.device, self.device_name.clone())),
            message: message.to_owned(),
            action,
            target: AuditTarget::File(ticket.device),
            change: json!({
                "path": transfer.path.as_str(),
                "size": transfer.size,
                "bytes": transfer.moved,
                "outcome": outcome.as_str(),
                "reason": reason.map(Refusal::as_str),
                "sessionId": ticket.session.to_string(),
            }),
        };
        let actor = (ticket.operator.clone(), ticket.client);
        let trail = events::Trail::new(&self.bus, actor, self.clock.now());
        let mut tx = self.db.begin().await?;
        trail.write(&mut tx, entry).await?;
        tx.commit().await?;
        Ok(())
    }
}

/// The sentence of a files tunnel refused because the device's host key changed (SEC-007),
/// in the words of the terminal's `HOST_KEY_MISMATCH` end.
pub(super) const HOST_KEY_CHANGED: &str = "The device presented an SSH host key that is not the \
                                           accepted one, so the hub opened no files tunnel \
                                           and sent no credential. Test the connection and \
                                           check the new key before accepting it.";

/// Writes the Error event of a files tunnel to `device` refused at `now` because the device
/// presented another host key than the accepted one.
///
/// # Errors
/// `Database` or `Events` when the event cannot be written.
pub(super) async fn host_key_changed(
    db: &PgPool,
    bus: &Bus,
    now: DateTime<Utc>,
    device: &Device,
) -> Result<(), Error> {
    let event = NewEvent {
        severity: Severity::Error,
        source: Source::Session,
        device: Some((device.id, device.name.clone())),
        message: HOST_KEY_CHANGED.to_owned(),
        // The hub refuses the device's key by itself (SEC-007), so the event is Hub (LOG-013).
        actor: ActorKind::Hub,
    };
    let mut tx = db.begin().await?;
    events::record(&mut tx, bus, now, event).await?;
    tx.commit().await?;
    Ok(())
}
