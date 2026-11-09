//! What a device mutation leaves behind: one event and one audit entry, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007).
//!
//! The messages and the `change` objects are built by pure functions from the stored rows and the
//! validated input. They hold names, hosts and settings, never a secret.

use std::num::NonZeroU8;

use serde_json::{Value, json};

use super::credential_input::CredentialKind;
use crate::{
    credentials::PasswordKind,
    devices::{
        RdpSettings, TransferPolicy, VncSettings,
        model::{Device, DeviceChange, NewDevice, Site, SshSettings},
    },
    events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
};

fn ssh_json(ssh: &SshSettings) -> Value {
    // `signIn`, not `auth`: the audit log hides values under keys that name a credential.
    json!({
        "enabled": ssh.enabled,
        "port": ssh.port,
        "username": ssh.username,
        "signIn": ssh.auth.as_str(),
    })
}

fn rdp_json(rdp: &RdpSettings) -> Value {
    json!({
        "enabled": rdp.enabled,
        "port": rdp.port,
        "username": rdp.username,
        "domain": rdp.domain,
        "security": rdp.security.as_str(),
        "ignoreCertificate": rdp.ignore_certificate,
        "resizeMethod": rdp.resize.as_str(),
        "audioOut": rdp.audio_out,
        "microphone": rdp.microphone,
        "sharedDrive": rdp.shared_drive,
    })
}

fn vnc_json(vnc: &VncSettings) -> Value {
    json!({ "enabled": vnc.enabled, "port": vnc.port, "username": vnc.username })
}

fn transfer_json(transfer: TransferPolicy) -> Value {
    json!({
        "upload": transfer.upload,
        "download": transfer.download,
        "copy": transfer.copy,
        "paste": transfer.paste,
    })
}

pub(super) fn site_added(site: &Site) -> TrailEntry {
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: None,
        message: format!("Site {} added.", site.name),
        action: AuditAction::Create,
        target: AuditTarget::Site(site.id),
        change: json!({ "name": site.name, "subnet": site.subnet.to_string() }),
    }
}

pub(super) fn device_added(device: &Device, new: &NewDevice, default_check: bool) -> TrailEntry {
    let mut change = json!({
        "siteId": new.site.to_string(),
        "name": new.name,
        "host": new.host,
        "kind": new.kind,
        "os": new.os.as_str(),
        "tags": new.tags,
        "ssh": ssh_json(&new.ssh),
        "rdp": rdp_json(&new.remote.rdp),
        "vnc": vnc_json(&new.remote.vnc),
        "macAddress": new.remote.mac_address.map(|mac| mac.to_string()),
        "metricsSource": new.remote.metrics_source.as_str(),
        "transfer": transfer_json(new.remote.transfer),
        "sessionLimit": new.remote.session_limit.map(NonZeroU8::get),
        "recordSessions": new.remote.record_sessions,
    });
    if default_check {
        change["defaultCheck"] = json!(true);
    }
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Device {} added.", device.name),
        action: AuditAction::Create,
        target: AuditTarget::Device(device.id),
        change,
    }
}

/// Only the fields the operator sent appear in `change`.
pub(super) fn device_updated(device: &Device, change: &DeviceChange) -> TrailEntry {
    let mut fields = serde_json::Map::new();
    let mut put = |key: &str, value: Option<Value>| {
        if let Some(value) = value {
            fields.insert(key.to_owned(), value);
        }
    };
    put("siteId", change.site.map(|site| json!(site.to_string())));
    put("name", change.name.as_ref().map(|v| json!(v)));
    put("host", change.host.as_ref().map(|v| json!(v)));
    put("kind", change.kind.as_ref().map(|v| json!(v)));
    put("os", change.os.map(|os| json!(os.as_str())));
    put("tags", change.tags.as_ref().map(|v| json!(v)));
    put("maintenance", change.maintenance.map(|v| json!(v)));
    put("ssh", change.ssh.as_ref().map(ssh_json));
    put("rdp", change.remote.rdp.as_ref().map(rdp_json));
    put("vnc", change.remote.vnc.as_ref().map(vnc_json));
    // A cleared address is recorded as null.
    put(
        "macAddress",
        change
            .remote
            .mac_address
            .map(|mac| json!(mac.map(|mac| mac.to_string()))),
    );
    put(
        "metricsSource",
        change
            .remote
            .metrics_source
            .map(|source| json!(source.as_str())),
    );
    put("transfer", change.remote.transfer.map(transfer_json));
    // A cleared limit is recorded as null.
    put(
        "sessionLimit",
        change
            .remote
            .session_limit
            .map(|limit| json!(limit.map(NonZeroU8::get))),
    );
    put(
        "recordSessions",
        change.remote.record_sessions.map(|v| json!(v)),
    );
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Device {} updated.", device.name),
        action: AuditAction::Update,
        target: AuditTarget::Device(device.id),
        change: Value::Object(fields),
    }
}

/// The deleted device's name, host and site, so the entry outlives the device (DEV-015).
pub(super) fn device_deleted(device: &Device) -> TrailEntry {
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Device {} deleted.", device.name),
        action: AuditAction::Delete,
        target: AuditTarget::Device(device.id),
        change: json!({
            "name": device.name,
            "host": device.host,
            "siteId": device.site.to_string(),
        }),
    }
}

/// Names the kind of secret and whether the key is encrypted, never the secret. The flag is
/// called `encrypted` because the audit log hides the value of any key that names a passphrase.
pub(super) fn credential_set(device: &Device, kind: CredentialKind) -> TrailEntry {
    let change = match kind {
        CredentialKind::Password => json!({ "kind": "password" }),
        CredentialKind::PrivateKey { passphrase } => {
            json!({ "kind": "privateKey", "encrypted": passphrase })
        }
    };
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Device {} SSH credential set.", device.name),
        action: AuditAction::Update,
        target: AuditTarget::Credential(device.id),
        change,
    }
}

pub(super) fn credential_cleared(device: &Device) -> TrailEntry {
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Device {} SSH credential cleared.", device.name),
        action: AuditAction::Delete,
        target: AuditTarget::Credential(device.id),
        change: json!({ "cleared": true }),
    }
}

/// The protocol a password is for: as audit text and as message text.
const fn protocol(kind: PasswordKind) -> (&'static str, &'static str) {
    match kind {
        PasswordKind::Rdp => ("rdp", "RDP"),
        PasswordKind::Vnc => ("vnc", "VNC"),
    }
}

/// Names the protocol and that a password was stored, never the password.
pub(super) fn password_set(device: &Device, kind: PasswordKind) -> TrailEntry {
    let (audit, message) = protocol(kind);
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Device {} {message} password set.", device.name),
        action: AuditAction::Update,
        target: AuditTarget::Credential(device.id),
        change: json!({ "protocol": audit, "kind": "password" }),
    }
}

pub(super) fn password_cleared(device: &Device, kind: PasswordKind) -> TrailEntry {
    let (audit, message) = protocol(kind);
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Device {} {message} password cleared.", device.name),
        action: AuditAction::Delete,
        target: AuditTarget::Credential(device.id),
        change: json!({ "protocol": audit, "kind": "password", "cleared": true }),
    }
}

/// The site's new wake relay, by id and name, or null when it was cleared (D-7).
pub(super) fn wake_relay_changed(site: &Site, relay: Option<&Device>) -> TrailEntry {
    let (device, message, change) = match relay {
        Some(relay) => (
            Some((relay.id, relay.name.clone())),
            format!("Site {} wake relay set to {}.", site.name, relay.name),
            json!({ "id": relay.id.to_string(), "name": relay.name }),
        ),
        None => (
            None,
            format!("Site {} wake relay cleared.", site.name),
            Value::Null,
        ),
    };
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device,
        message,
        action: AuditAction::Update,
        target: AuditTarget::Site(site.id),
        change: json!({ "wakeRelay": change }),
    }
}
