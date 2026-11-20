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

/// What the audit entry of a batch of an Anima phone app holds: the counts and the name of the
/// operator, never a position. A stored sample writes no event (R-NEW-locationhours).
pub(super) fn recorded_change(origin: &Origin, origin_name: &str, stored: Stored) -> Value {
    json!({
        "provider": origin.provider.as_str(),
        "from": origin_name,
        "added": stored.added,
        "known": stored.known,
    })
}

/// What the audit entry of one hour of one ingest token holds (R-NEW-locationhours): the name of the token,
/// the start of the hour and the count of the fixes that the hub stored in it.
pub(super) fn hour_change(name: &str, hour: DateTime<Utc>, fixes: i32) -> Value {
    json!({ "name": name, "provider": "owntracks", "hour": hour, "fixes": fixes })
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
