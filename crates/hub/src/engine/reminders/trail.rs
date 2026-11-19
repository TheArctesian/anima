//! The audit entries and events of a change of a rule (LOG-004, LOG-005, LOG-007). Pure. An
//! entry holds the name of the rule, the type of its trigger and its limits: never its message
//! and never a title or a value of a record.

use serde_json::{Value, json};

use super::{store::StoredRule, trigger_json};
use crate::fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry};

fn change(stored: &StoredRule) -> Value {
    let rule = &stored.rule;
    json!({
        "name": rule.name,
        "trigger": trigger_json(&rule.trigger)["type"],
        "cooldownMinutes": rule.limits.cooldown_minutes,
        "dailyLimit": rule.limits.daily_limit,
        "channels": stored.channels.len(),
        "enabled": stored.enabled,
        "revision": rule.revision,
    })
}

fn entry(action: AuditAction, stored: &StoredRule, verb: &str) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: format!("The reminder rule {} was {verb}.", stored.rule.name),
        action,
        target: AuditTarget::ReminderRule(stored.rule.id),
        change: change(stored),
    }
}

pub(super) fn created(stored: &StoredRule) -> TrailEntry {
    entry(AuditAction::Create, stored, "made")
}

pub(super) fn updated(stored: &StoredRule) -> TrailEntry {
    entry(AuditAction::Update, stored, "changed")
}

pub(super) fn removed(stored: &StoredRule) -> TrailEntry {
    entry(AuditAction::Delete, stored, "deleted")
}

pub(super) fn switched(stored: &StoredRule) -> TrailEntry {
    if stored.enabled {
        entry(AuditAction::Activate, stored, "set on")
    } else {
        entry(AuditAction::Deactivate, stored, "set off")
    }
}
