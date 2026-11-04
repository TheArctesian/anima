//! What a check mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007).
//!
//! The `change` objects are built by pure functions from the stored checks. A secret appears
//! only as `"[set]"` under the key `sealed`; no URL, body or header value is recorded.

use std::sync::Arc;

use async_graphql::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use sqlx::PgConnection;

use super::error::{internal, schema_dependency};
use crate::{
    checks::{
        Check,
        config::{CheckConfig, HttpAuth},
    },
    clock::Clock,
    events::{
        self, AuditAction, AuditTarget, Bus, ClientAddr, NewAudit, NewEvent, Operator, Severity,
        Source,
    },
    ids::{CheckId, DeviceId},
};

/// One mutation's entry in both logs.
pub(super) struct Entry {
    device: (DeviceId, String),
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
            source: Source::Check,
            device: Some(entry.device),
            message: entry.message,
        };
        events::record(conn, self.bus, self.now, event)
            .await
            .map(drop)
            .map_err(internal)
    }
}

/// What the configuration says without its secrets: the type, and for HTTP only the method,
/// the sign-in kind and the header names.
fn config_json(config: &CheckConfig) -> Value {
    match config {
        CheckConfig::Http(http) => {
            let sign_in = match http.auth {
                HttpAuth::None => "none",
                HttpAuth::Basic { .. } => "basic",
                HttpAuth::Bearer => "bearer",
            };
            let names: Vec<&str> = http.headers.iter().map(|h| h.name.as_str()).collect();
            json!({
                "type": "http",
                "origin": reqwest::Url::parse(&http.url).map(|url| url.origin().ascii_serialization()).ok(),
                "method": http.method,
                "signIn": sign_in,
                "headerNames": names,
            })
        }
        CheckConfig::Push => json!({ "type": "push" }),
        other => serde_json::to_value(other).unwrap_or(Value::Null),
    }
}

fn fields(check: &Check) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert("name".into(), json!(check.name));
    fields.insert(
        "parentId".into(),
        json!(check.parent.map(|id| id.to_string())),
    );
    fields.insert(
        "intervalSeconds".into(),
        json!(check.settings.interval_seconds),
    );
    fields.insert("retries".into(), json!(check.settings.retries));
    fields.insert(
        "retryIntervalSeconds".into(),
        json!(check.settings.retry_interval_seconds),
    );
    fields.insert(
        "degradedThresholdMs".into(),
        json!(check.settings.degraded_threshold_ms),
    );
    fields.insert("invert".into(), json!(check.settings.invert));
    fields.insert("config".into(), config_json(&check.config));
    fields
}

fn entry(check: &Check, device: &str, action: AuditAction, (verb, change): (&str, Value)) -> Entry {
    Entry {
        device: (check.device, device.to_owned()),
        message: format!("Check {} on {device} {verb}.", check.name),
        action,
        target: AuditTarget::Check(check.id),
        change,
    }
}

fn child_ids(children: &[CheckId]) -> Value {
    json!(children.iter().map(ToString::to_string).collect::<Vec<_>>())
}

/// What a mutation did besides the plain fields.
pub(super) struct Extras<'a> {
    /// The operator sent a non-empty secret.
    pub(super) sealed: bool,
    /// A group's new set of children.
    pub(super) children: Option<&'a [CheckId]>,
    /// The push check got a new token (never recorded).
    pub(super) rotated: bool,
}

fn add_extras(change: &mut Map<String, Value>, extras: &Extras<'_>) {
    if extras.sealed {
        change.insert("sealed".into(), json!("[set]"));
    }
    if let Some(children) = extras.children {
        change.insert("childIds".into(), child_ids(children));
    }
    if extras.rotated {
        change.insert("pushPath".into(), json!("[changed]"));
    }
}

pub(super) fn created(check: &Check, device: &str, extras: &Extras<'_>) -> Entry {
    let mut change = fields(check);
    change.insert("deviceId".into(), json!(check.device.to_string()));
    add_extras(&mut change, extras);
    entry(
        check,
        device,
        AuditAction::Create,
        ("added", Value::Object(change)),
    )
}

/// Only the fields that differ appear in `change`.
pub(super) fn updated(before: &Check, after: &Check, device: &str, extras: &Extras<'_>) -> Entry {
    let (old, new) = (fields(before), fields(after));
    let mut change = Map::new();
    for (key, value) in &new {
        if old.get(key) != Some(value) {
            change.insert(key.clone(), value.clone());
        }
    }
    add_extras(&mut change, extras);
    entry(
        after,
        device,
        AuditAction::Update,
        ("updated", Value::Object(change)),
    )
}

/// The deleted check's name and type, so the entry outlives the check.
pub(super) fn deleted(check: &Check, device: &str) -> Entry {
    let change = json!({ "name": check.name, "type": check.kind().as_sql() });
    entry(check, device, AuditAction::Delete, ("deleted", change))
}

pub(super) fn paused(check: &Check, device: &str) -> Entry {
    let (verb, action) = if check.paused {
        ("paused", AuditAction::Pause)
    } else {
        ("resumed", AuditAction::Resume)
    };
    entry(
        check,
        device,
        action,
        (verb, json!({ "paused": check.paused })),
    )
}
