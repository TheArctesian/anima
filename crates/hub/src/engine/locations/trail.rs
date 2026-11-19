//! The audit entries and events of the locations domain (LOG-004, LOG-005, LOG-007). Pure. An
//! entry holds counts, times and names: never a position and never a token.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use super::model::{IngestToken, Origin, Stored};
use crate::fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry};

fn entry(action: AuditAction, target: AuditTarget, message: String, change: Value) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target,
        change,
    }
}

/// A batch of samples that the hub stored; `origin_name` is the name of the ingest token or
/// of the operator that gave it.
pub(super) fn recorded(origin: &Origin, origin_name: &str, stored: Stored) -> TrailEntry {
    entry(
        AuditAction::Create,
        AuditTarget::LocationSamples,
        format!(
            "The hub stored {} new location sample(s) from {}.",
            stored.added,
            origin.provider.as_str()
        ),
        json!({
            "provider": origin.provider.as_str(),
            "from": origin_name,
            "added": stored.added,
            "known": stored.known,
        }),
    )
}

/// What the audit entry of a read of samples holds: the span and the count.
pub(super) fn read_change((from, to): (DateTime<Utc>, DateTime<Utc>), count: usize) -> Value {
    json!({ "from": from, "to": to, "count": count })
}

fn ingest_change(token: &IngestToken) -> Value {
    json!({ "name": token.name })
}

pub(super) fn token_created(token: &IngestToken) -> TrailEntry {
    entry(
        AuditAction::Create,
        AuditTarget::IngestToken(token.id),
        format!("A location ingest token named {} was made.", token.name),
        ingest_change(token),
    )
}

pub(super) fn token_revoked(token: &IngestToken) -> TrailEntry {
    entry(
        AuditAction::Deactivate,
        AuditTarget::IngestToken(token.id),
        format!(
            "The location ingest token named {} was revoked.",
            token.name
        ),
        ingest_change(token),
    )
}
