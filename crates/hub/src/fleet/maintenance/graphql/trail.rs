//! What a window mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007).
//!
//! The event source is `Hub`: a window belongs to no single device. The `change` objects are
//! built by pure functions from the stored windows.

use serde_json::{Map, Value, json};

use crate::fleet::{
    events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    maintenance::Window,
};

/// The fields of a window that `change` can name, by their GraphQL names.
fn fields(window: &Window) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert("name".into(), json!(window.name));
    fields.insert("timezone".into(), json!(window.timezone.name()));
    if let Ok(Value::Object(rule)) = serde_json::to_value(&window.rule) {
        fields.extend(rule);
    }
    let targets: Vec<Value> = window
        .targets
        .iter()
        .map(|t| json!({ "type": t.target_type().as_str(), "target": t.value() }))
        .collect();
    fields.insert("targets".into(), Value::Array(targets));
    fields
}

pub(super) fn window_added(window: &Window) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: format!("Maintenance window {} added.", window.name),
        action: AuditAction::Create,
        target: AuditTarget::Window(window.id),
        change: Value::Object(fields(window)),
    }
}

/// Only the fields that differ appear in `change`; a field the new rule no longer has is null.
pub(super) fn window_updated(before: &Window, after: &Window) -> TrailEntry {
    let (old, new) = (fields(before), fields(after));
    let mut changed = Map::new();
    for key in old.keys().chain(new.keys()) {
        if old.get(key) != new.get(key) {
            changed.insert(key.clone(), new.get(key).cloned().unwrap_or(Value::Null));
        }
    }
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: format!("Maintenance window {} updated.", after.name),
        action: AuditAction::Update,
        target: AuditTarget::Window(after.id),
        change: Value::Object(changed),
    }
}

/// The deleted window's name and kind, so the entry outlives the window.
pub(super) fn window_deleted(window: &Window) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: format!("Maintenance window {} deleted.", window.name),
        action: AuditAction::Delete,
        target: AuditTarget::Window(window.id),
        change: json!({ "name": window.name, "kind": window.rule.kind().as_str() }),
    }
}

pub(super) fn manual_switched(window: &Window) -> TrailEntry {
    let (verb, action) = if window.active_manual {
        ("on", AuditAction::Activate)
    } else {
        ("off", AuditAction::Deactivate)
    };
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: format!("Maintenance window {} switched {verb}.", window.name),
        action,
        target: AuditTarget::Window(window.id),
        change: json!({ "activeManual": window.active_manual }),
    }
}
