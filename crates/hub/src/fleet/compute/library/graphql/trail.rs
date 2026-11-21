//! What a library mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007). The library has no id of its own
//! in the audit log; the target is the setting `compute.library`, and the change names the
//! model, the alias or the organisation.

use serde_json::json;

use super::super::{Model, PublisherApproval, Widening};
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

/// The message and the change name the publisher that joined the list, or that stayed out
/// because an API key or an agent added the model (R-NEW-modellibrary).
pub(super) fn model_added(model: &Model, repository: &str, widening: &Widening) -> TrailEntry {
    let tags: Vec<&str> = model
        .variants
        .iter()
        .map(|each| each.tag.as_str())
        .collect();
    let mut message = format!("Model {} read from {repository}.", model.name);
    let mut change = json!({ "model": model.name, "repository": repository, "tags": tags });
    match widening {
        Widening::Nothing => {}
        Widening::Add(organisation) => {
            message = format!("{message} Publisher {organisation} added to the approved sources.");
            change["sourceAdded"] = json!(organisation);
        }
        Widening::Withheld(organisation) => {
            message = format!(
                "{message} Publisher {organisation} not approved: only a person widens the approved sources."
            );
            change["sourceWithheld"] = json!(organisation);
        }
    }
    entry(AuditAction::Create, message, change)
}

pub(super) fn approval_set(mode: PublisherApproval) -> TrailEntry {
    entry(
        AuditAction::Update,
        format!("Model publisher approval set to {}.", mode.as_str()),
        json!({ "publisherApproval": mode.as_str() }),
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
