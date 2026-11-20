//! The audit entry and the event of each change (LOG-004, LOG-005). Pure. An entry names the
//! owner record, the extension and the count of the bytes, and never a byte (VLT-004).

use serde_json::json;

use super::Record;
use crate::{
    fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    ids::KeyOfferId,
};

fn entry(record: &Record, action: AuditAction, verb: &str) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: format!("The sealed record of {} was {verb}.", record.owner),
        action,
        target: AuditTarget::SealedRecord(record.id),
        change: json!({
            "owner": record.owner.as_str(),
            "extension": record.extension,
            "bytes": record.sealed.len(),
            "index_bytes": record.sealed_index.as_ref().map(Vec::len),
        }),
    }
}

pub(super) fn kept(record: &Record, made: bool) -> TrailEntry {
    if made {
        entry(record, AuditAction::Create, "stored")
    } else {
        entry(record, AuditAction::Update, "replaced")
    }
}

pub(super) fn removed(record: &Record) -> TrailEntry {
    entry(record, AuditAction::Delete, "deleted")
}

pub(super) fn offered(id: KeyOfferId, bytes: usize) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: "A client offered the vault key to a new client.".to_owned(),
        action: AuditAction::Create,
        target: AuditTarget::VaultKeyOffer(id),
        change: json!({ "bytes": bytes }),
    }
}
