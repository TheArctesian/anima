//! The audit entry and the event of each change of an operational key, and the audit entry of a
//! fetch (LOG-004, LOG-005; K-E5 Section 2.2). Pure. Each names the key, its metadata, its
//! version numbers and its consumers; none holds a value (LOG-007).

use serde_json::{Value, json};

use super::model::{Grant, Metadata};
use crate::{
    fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    ids::{DeviceId, OperationalKeyId},
};

fn entry(
    (id, action): (OperationalKeyId, AuditAction),
    severity: Severity,
    message: String,
    change: Value,
) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity,
        device: None,
        message,
        action,
        target: AuditTarget::OperationalKey(id),
        change,
    }
}

fn described(name: &str, metadata: &Metadata) -> Value {
    json!({
        "name": name,
        "provider": metadata.provider,
        "purpose": metadata.purpose,
        "providerLink": metadata.provider_link,
        "rotationDays": metadata.rotation_days,
        "expiresAt": metadata.expires_at,
    })
}

pub(super) fn created(id: OperationalKeyId, name: &str, metadata: &Metadata) -> TrailEntry {
    entry(
        (id, AuditAction::Create),
        Severity::Info,
        format!("Operational key {name} added with version 1."),
        described(name, metadata),
    )
}

pub(super) fn updated(id: OperationalKeyId, name: &str, metadata: &Metadata) -> TrailEntry {
    entry(
        (id, AuditAction::Update),
        Severity::Info,
        format!("Operational key {name} changed."),
        described(name, metadata),
    )
}

/// A new version: "recorded", never "rotated", unless the operator said the provider changed
/// it too (K-E5 Section 2).
pub(super) fn rotated(
    (id, name): (OperationalKeyId, &str),
    (ended, made): (Option<i32>, i32),
    rotated_at_provider: bool,
) -> TrailEntry {
    let what = if rotated_at_provider {
        "rotated at the provider and recorded"
    } else {
        "recorded; it was not rotated at the provider by Anima"
    };
    entry(
        (id, AuditAction::Update),
        Severity::Info,
        format!("Operational key {name}: version {made} {what}."),
        json!({
            "name": name,
            "version": made,
            "supersededVersion": ended,
            "rotatedAtProvider": rotated_at_provider,
        }),
    )
}

pub(super) fn revoked(
    (id, name): (OperationalKeyId, &str),
    ended: Option<i32>,
    holders: usize,
) -> TrailEntry {
    entry(
        (id, AuditAction::Deactivate),
        Severity::Warning,
        format!(
            "Operational key {name} revoked; {holders} consumers held the ended version at their \
             last report."
        ),
        json!({ "name": name, "revokedVersion": ended }),
    )
}

pub(super) fn removed(
    (id, name): (OperationalKeyId, &str),
    ended: Option<i32>,
    holders: usize,
) -> TrailEntry {
    entry(
        (id, AuditAction::Delete),
        Severity::Warning,
        format!(
            "Operational key {name} removed; {holders} consumers held the ended version at their \
             last report."
        ),
        json!({ "name": name, "revokedVersion": ended }),
    )
}

pub(super) fn granted(id: OperationalKeyId, grant: &Grant) -> TrailEntry {
    entry(
        (id, AuditAction::Activate),
        Severity::Info,
        format!(
            "Operational key {} granted to device {} for {}.",
            grant.name, grant.device, grant.workload
        ),
        json!({ "name": grant.name, "device": grant.device, "workload": grant.workload }),
    )
}

pub(super) fn ungranted(id: OperationalKeyId, grant: &Grant) -> TrailEntry {
    entry(
        (id, AuditAction::Deactivate),
        Severity::Info,
        format!(
            "Operational key {} no longer granted to device {} for {}.",
            grant.name, grant.device, grant.workload
        ),
        json!({ "name": grant.name, "device": grant.device, "workload": grant.workload }),
    )
}

/// The change of a fetch's audit entry: the key, the version and the device (K-E5 Section 2.1).
pub(super) fn fetched(name: &str, version: i32, device: DeviceId) -> Value {
    json!({ "name": name, "version": version, "device": device })
}

/// A device was removed while its server had `version` of the key: the hub cannot take the
/// value back from that server.
pub(super) fn device_removed(
    (id, name): (OperationalKeyId, &str),
    version: i32,
    device: DeviceId,
) -> TrailEntry {
    entry(
        (id, AuditAction::Update),
        Severity::Warning,
        format!(
            "A device that had version {version} of operational key {name} was removed. The hub \
             cannot take the value back: revoke the key at its provider if the server is lost."
        ),
        json!({ "name": name, "heldVersion": version, "removedDevice": device }),
    )
}
