//! What an attempt leaves in the audit log and the event log before an account exists: started,
//! App created and cancelled, each on the setting `sources.github.connect` (K-E1 Section 7a,
//! LOG-004). An entry holds the label, the organisation and the actor, or the App's ID, slug
//! and owner; never a key, a code, a ticket, a cookie or a state (LOG-007, PRJ-041).

use serde_json::json;

use super::attempt::App;
use crate::fleet::events::{ActorKind, AuditAction, AuditTarget, Severity, Source, TrailEntry};

/// The audit target of every entry before the account exists.
const TARGET: AuditTarget = AuditTarget::Setting("sources.github.connect");

fn entry(action: AuditAction, message: &str, change: serde_json::Value) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: message.to_owned(),
        action,
        target: TARGET,
        change,
    }
}

/// An attempt was started by `actor`.
pub fn started(label: Option<&str>, organisation: Option<&str>, actor: ActorKind) -> TrailEntry {
    entry(
        AuditAction::Start,
        "A GitHub connection was started.",
        json!({ "label": label, "organisation": organisation, "actor": actor.as_str() }),
    )
}

/// GitHub made `app`. The entry is what lets the operator delete it after a hub restart.
pub fn app_created(app: &App) -> TrailEntry {
    entry(
        AuditAction::Create,
        "A GitHub App was made for Anima.",
        json!({ "appId": app.id.to_string(), "slug": app.slug, "ownerLogin": app.owner.login }),
    )
}

/// The operator cancelled the attempt.
pub fn cancelled() -> TrailEntry {
    entry(
        AuditAction::Cancel,
        "A GitHub connection was cancelled.",
        json!({}),
    )
}
