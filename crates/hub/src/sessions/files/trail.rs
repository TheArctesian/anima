//! What each transfer leaves behind (K4 Section 9, LOG-005, LOG-007): one audit entry
//! `file.upload` or `file.download` by the ticket's operator and client address, with the path,
//! the size, the bytes moved and the outcome, never content; and one event with a fixed
//! sentence. The path stays in the audit entry: the event log is read more widely.

use std::sync::Arc;

use serde_json::json;
use sqlx::PgPool;

use super::{
    protocol::Refusal,
    tickets::FilesTicket,
    transfer::{Direction, Outcome, Transfer},
};
use crate::{
    clock::Clock,
    events::{self, AuditAction, AuditTarget, Bus, NewAudit, NewEvent, Severity, Source},
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
        let audit = NewAudit {
            operator: ticket.operator.clone(),
            action,
            target: AuditTarget::File(ticket.device),
            client: ticket.client,
            change: json!({
                "path": transfer.path.as_str(),
                "size": transfer.size,
                "bytes": transfer.moved,
                "outcome": outcome.as_str(),
                "reason": reason.map(Refusal::as_str),
                "sessionId": ticket.session.to_string(),
            }),
        };
        let (severity, message) = sentence(transfer.direction, outcome);
        let event = NewEvent {
            severity,
            source: Source::Session,
            device: Some((ticket.device, self.device_name.clone())),
            message: message.to_owned(),
        };
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        events::audit(&mut tx, now, audit).await?;
        events::record(&mut tx, &self.bus, now, event).await?;
        tx.commit().await?;
        Ok(())
    }
}
