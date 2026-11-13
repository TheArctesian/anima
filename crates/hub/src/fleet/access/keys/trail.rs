//! What an API key change leaves behind: one audit entry on the key and one `ACCESS` event
//! (LOG-004, LOG-005). The change holds the name, the scope and the expiry; never the secret,
//! its hash or its prefix (LOG-007).

use serde_json::json;

use super::ApiKey;
use crate::{
    fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    ids::ApiKeyId,
};

fn entry(
    id: ApiKeyId,
    action: AuditAction,
    message: String,
    change: serde_json::Value,
) -> TrailEntry {
    TrailEntry {
        source: Source::Access,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target: AuditTarget::ApiKey(id),
        change,
    }
}

pub(super) fn created(key: &ApiKey) -> TrailEntry {
    entry(
        key.id,
        AuditAction::Create,
        format!("API key {} created.", key.name),
        json!({
            "name": key.name,
            "scope": key.scope.as_str(),
            "madeFor": key.made_for.as_str(),
            "expiresAt": key.expires_at,
        }),
    )
}

/// Revoking deletes the key (D-23), so the audit action is `delete`.
pub(super) fn revoked(id: ApiKeyId, name: &str) -> TrailEntry {
    entry(
        id,
        AuditAction::Delete,
        format!("API key {name} revoked."),
        json!({ "name": name }),
    )
}
