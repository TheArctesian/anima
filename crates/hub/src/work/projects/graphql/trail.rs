//! What an operator's choice about a project leaves behind: one audit entry and one event, in
//! the mutation's own transaction (LOG-004, LOG-005). The words name the project by its slug,
//! which the hub made, never by text from GitHub (LOG-007).

use serde_json::json;

use crate::{
    fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    ids::{ProjectId, RepositoryId, ResourceId},
    work::projects::model::ResourceKind,
};

fn entry(action: AuditAction, project: ProjectId, message: String) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target: AuditTarget::Project(project),
        change: json!({}),
    }
}

/// A repository of the project `slug` was left out (PRO-002), or restored.
pub(super) fn left_out(
    (project, slug): (ProjectId, &str),
    repository: RepositoryId,
    left_out: bool,
) -> TrailEntry {
    let (action, words) = if left_out {
        (AuditAction::Deactivate, "left out of")
    } else {
        (AuditAction::Activate, "restored to")
    };
    TrailEntry {
        change: json!({ "repository": repository.to_string(), "leftOut": left_out }),
        ..entry(
            action,
            project,
            format!("A repository was {words} project {slug}."),
        )
    }
}

/// The operator confirmed a proposed resource of the project `slug` (PRO-004).
pub(super) fn confirmed(
    (project, slug): (ProjectId, &str),
    resource: ResourceId,
    kind: ResourceKind,
) -> TrailEntry {
    TrailEntry {
        change: json!({ "resource": resource.to_string(), "kind": kind.as_str(),
            "state": "confirmed" }),
        ..entry(
            AuditAction::Accept,
            project,
            format!("A resource of project {slug} was confirmed."),
        )
    }
}
