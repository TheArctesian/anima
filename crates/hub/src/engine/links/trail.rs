//! The audit entry and event of each link change (LOG-004, LOG-005). Pure. The event names
//! the two references and never the label, which may be a person's name.

use serde_json::json;

use super::model::{Link, Outcome};
use crate::fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry};

fn entry(link: &Link, action: AuditAction, verb: &str) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: format!(
            "Record link {verb}: {} {} {}.",
            link.source,
            link.relation.as_str(),
            link.target
        ),
        action,
        target: AuditTarget::RecordLink(link.id),
        change: json!({
            "source": link.source.as_str(),
            "target": link.target.as_str(),
            "kind": link.relation.as_str(),
            "label": link.label,
        }),
    }
}

/// The entry of a `connect`; `None` when it wrote nothing.
pub(super) fn connected(link: &Link, outcome: Outcome) -> Option<TrailEntry> {
    match outcome {
        Outcome::Made => Some(entry(link, AuditAction::Create, "added")),
        Outcome::Relabelled => Some(entry(link, AuditAction::Update, "relabelled")),
        Outcome::Kept => None,
    }
}

pub(super) fn disconnected(link: &Link) -> TrailEntry {
    entry(link, AuditAction::Delete, "removed")
}
