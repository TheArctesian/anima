//! What a library mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007). The library has no id of its own
//! in the audit log; the target is the setting `compute.library`, and the change names the
//! model, the alias or the organisation.

use serde_json::json;

use super::super::Model;
use crate::fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry};

const TARGET: AuditTarget = AuditTarget::Setting("compute.library");

fn entry(action: AuditAction, message: String, change: serde_json::Value) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target: TARGET,
        change,
    }
}

pub(super) fn model_added(model: &Model, repository: &str) -> TrailEntry {
    let tags: Vec<&str> = model
        .variants
        .iter()
        .map(|each| each.tag.as_str())
        .collect();
    entry(
        AuditAction::Create,
        format!("Model {} read from {repository}.", model.name),
        json!({ "model": model.name, "repository": repository, "tags": tags }),
    )
}

pub(super) fn alias_set(alias: &str, model: &str, before: Option<&str>) -> TrailEntry {
    entry(
        AuditAction::Update,
        format!("Alias {alias} points at {model}."),
        json!({ "alias": alias, "model": model, "previousModel": before }),
    )
}

pub(super) fn source_added(organisation: &str) -> TrailEntry {
    entry(
        AuditAction::Create,
        format!("Approved model source {organisation} added."),
        json!({ "organisation": organisation }),
    )
}
