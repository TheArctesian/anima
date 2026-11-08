//! What a channel mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007).
//!
//! The `change` objects are built by pure functions from the stored channels, so they hold
//! only what the API shows: the settings without secrets (a webhook's origin and header names,
//! never its URL or a header value). A secret appears only by its field name, under
//! `sealedSet` or `sealedCleared` (as checks records `sealed`: a key with "secret" in its name
//! is redacted whole). The event names the channel and nothing of its target.

use serde_json::{Map, Value, json};

use super::node::ChannelNode;
use crate::{
    events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    notifications::channels::Touched,
};

/// The audited fields of a channel; `settings` is the stored, secret-free JSON.
fn fields(node: &ChannelNode) -> Map<String, Value> {
    let fields = &node.channel.fields;
    let template = fields
        .title_template
        .as_ref()
        .zip(fields.body_template.as_ref())
        .map(|(title, body)| json!({ "title": title, "body": body }));
    let mut map = Map::new();
    map.insert("name".into(), json!(fields.name));
    map.insert("kind".into(), json!(fields.kind.as_str()));
    map.insert("onByDefault".into(), json!(fields.on_by_default));
    map.insert("settings".into(), node.settings.to_stored());
    map.insert("template".into(), template.unwrap_or(Value::Null));
    map
}

/// The entries of `new` whose value differs from `old`'s.
fn differing(old: &Map<String, Value>, new: &Map<String, Value>) -> Map<String, Value> {
    new.iter()
        .filter(|(key, value)| old.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn add_touched(change: &mut Map<String, Value>, touched: &Touched) {
    if !touched.set.is_empty() {
        change.insert("sealedSet".into(), json!(touched.set));
    }
    if !touched.cleared.is_empty() {
        change.insert("sealedCleared".into(), json!(touched.cleared));
    }
}

fn entry(node: &ChannelNode, action: AuditAction, (verb, change): (&str, Value)) -> TrailEntry {
    TrailEntry {
        source: Source::Notifications,
        severity: Severity::Info,
        device: None,
        message: format!("Notification channel {} {verb}.", node.channel.fields.name),
        action,
        target: AuditTarget::Channel(node.channel.id),
        change,
    }
}

pub(super) fn created(node: &ChannelNode, touched: &Touched) -> TrailEntry {
    let mut change = fields(node);
    add_touched(&mut change, touched);
    entry(node, AuditAction::Create, ("added", Value::Object(change)))
}

/// Only the fields that differ appear in `change`; within `settings`, only the settings that
/// differ.
pub(super) fn updated(before: &ChannelNode, after: &ChannelNode, touched: &Touched) -> TrailEntry {
    let (old, new) = (fields(before), fields(after));
    let mut change = differing(&old, &new);
    if let (Some(Value::Object(old)), Some(Value::Object(new))) =
        (old.get("settings"), new.get("settings"))
    {
        let settings = differing(old, new);
        if settings.is_empty() {
            change.remove("settings");
        } else {
            change.insert("settings".into(), Value::Object(settings));
        }
    }
    add_touched(&mut change, touched);
    entry(
        after,
        AuditAction::Update,
        ("updated", Value::Object(change)),
    )
}

/// The deleted channel's name and kind, so the entry outlives the channel.
pub(super) fn deleted(node: &ChannelNode) -> TrailEntry {
    let fields = &node.channel.fields;
    let change = json!({ "name": fields.name, "kind": fields.kind.as_str() });
    entry(node, AuditAction::Delete, ("deleted", change))
}
