//! What an operator's change to an account leaves behind: one audit entry and one event, in
//! the mutation's own transaction (LOG-004, LOG-005). The change names the account and never
//! holds the key or a token (LOG-007, PRJ-018).

use serde_json::json;

use crate::{
    engine::sources::model::Account,
    fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    ids::SourceAccountId,
};

fn entry(action: AuditAction, target: SourceAccountId, message: String) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target: AuditTarget::SourceAccount(target),
        change: json!({}),
    }
}

/// An account was connected, or its key replaced (`created` false).
pub(super) fn connected(
    id: SourceAccountId,
    (source, label, instance_key): (&str, &str, &str),
    created: bool,
) -> TrailEntry {
    let (action, words) = if created {
        (AuditAction::Create, "connected")
    } else {
        (AuditAction::Update, "given a new key")
    };
    TrailEntry {
        change: json!({ "source": source, "label": label, "instanceKey": instance_key,
            "keyStored": true }),
        ..entry(action, id, format!("Source account {label} {words}."))
    }
}

/// An account was paused or resumed.
pub(super) fn paused(account: &Account, paused: bool) -> TrailEntry {
    let (action, words) = if paused {
        (AuditAction::Pause, "paused")
    } else {
        (AuditAction::Resume, "resumed")
    };
    TrailEntry {
        change: json!({ "paused": paused }),
        ..entry(
            action,
            account.id,
            format!("Source account {} {words}.", account.label),
        )
    }
}

/// An account was removed, with or without the records it imported (ITG-004).
pub(super) fn removed(account: &Account, delete_records: bool, deleted: u64) -> TrailEntry {
    let words = if delete_records {
        "removed with its records"
    } else {
        "removed; its records are kept"
    };
    TrailEntry {
        change: json!({ "source": account.source, "label": account.label,
            "instanceKey": account.instance_key, "deleteRecords": delete_records,
            "recordsDeleted": deleted }),
        ..entry(
            AuditAction::Delete,
            account.id,
            format!("Source account {} {words}.", account.label),
        )
    }
}
