//! What a window mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007).
//!
//! The event source is `Hub`: a window belongs to no single device. The `change` objects are
//! built by pure functions from the stored windows.

use std::sync::Arc;

use async_graphql::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use sqlx::PgConnection;

use crate::{
    api_errors::{internal, schema_dependency},
    clock::Clock,
    events::{
        self, AuditAction, AuditTarget, Bus, ClientAddr, NewAudit, NewEvent, Operator, Severity,
        Source,
    },
    maintenance::Window,
};

/// One mutation's entry in both logs.
pub(super) struct Entry {
    message: String,
    action: AuditAction,
    target: AuditTarget,
    change: Value,
}

/// Who did it, from where and when; the same `at` stamps the store write and both entries.
pub(super) struct Trail<'a> {
    bus: &'a Bus,
    operator: Operator,
    client: ClientAddr,
    pub(super) now: DateTime<Utc>,
}

impl<'a> Trail<'a> {
    /// Reads the request's operator and address and the schema's bus and clock. The clock is
    /// read once.
    pub(super) fn from_context(ctx: &Context<'a>) -> Result<Self> {
        Ok(Self {
            bus: schema_dependency::<Bus>(ctx)?,
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
            now: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        })
    }

    /// Writes the audit entry, then the event. The caller commits both with its own write.
    ///
    /// The event goes last because `events::record` publishes it on the bus at once: if the
    /// audit write fails, no subscriber has seen an event for a change that is rolled back.
    pub(super) async fn write(self, conn: &mut PgConnection, entry: Entry) -> Result<()> {
        let audit = NewAudit {
            operator: self.operator,
            action: entry.action,
            target: entry.target,
            client: self.client,
            change: entry.change,
        };
        events::audit(conn, self.now, audit)
            .await
            .map_err(internal)?;
        let event = NewEvent {
            severity: Severity::Info,
            source: Source::Hub,
            device: None,
            message: entry.message,
        };
        events::record(conn, self.bus, self.now, event)
            .await
            .map(drop)
            .map_err(internal)
    }
}

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

pub(super) fn window_added(window: &Window) -> Entry {
    Entry {
        message: format!("Maintenance window {} added.", window.name),
        action: AuditAction::Create,
        target: AuditTarget::Window(window.id),
        change: Value::Object(fields(window)),
    }
}

/// Only the fields that differ appear in `change`; a field the new rule no longer has is null.
pub(super) fn window_updated(before: &Window, after: &Window) -> Entry {
    let (old, new) = (fields(before), fields(after));
    let mut changed = Map::new();
    for key in old.keys().chain(new.keys()) {
        if old.get(key) != new.get(key) {
            changed.insert(key.clone(), new.get(key).cloned().unwrap_or(Value::Null));
        }
    }
    Entry {
        message: format!("Maintenance window {} updated.", after.name),
        action: AuditAction::Update,
        target: AuditTarget::Window(after.id),
        change: Value::Object(changed),
    }
}

/// The deleted window's name and kind, so the entry outlives the window.
pub(super) fn window_deleted(window: &Window) -> Entry {
    Entry {
        message: format!("Maintenance window {} deleted.", window.name),
        action: AuditAction::Delete,
        target: AuditTarget::Window(window.id),
        change: json!({ "name": window.name, "kind": window.rule.kind().as_str() }),
    }
}

pub(super) fn manual_switched(window: &Window) -> Entry {
    let (verb, action) = if window.active_manual {
        ("on", AuditAction::Activate)
    } else {
        ("off", AuditAction::Deactivate)
    };
    Entry {
        message: format!("Maintenance window {} switched {verb}.", window.name),
        action,
        target: AuditTarget::Window(window.id),
        change: json!({ "activeManual": window.active_manual }),
    }
}
