//! What a web interface mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007, WEB-009).
//!
//! The audit target is the device (D-3). A `change` holds the interface id, the URL's origin
//! and a fingerprint, never the URL itself: a URL's path and query may carry a token. The
//! event names the interface by its label and carries the device (CR-5).

use reqwest::Url;
use serde_json::{Value, json};

use crate::{
    fleet::{
        devices::Device,
        events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
        web_interfaces::WebInterface,
    },
    ids::{CheckId, WebInterfaceId},
};

/// `scheme://host:port` of a URL: no user name, path or query. Empty when it does not parse.
fn origin(url: &str) -> String {
    Url::parse(url).map_or_else(|_| String::new(), |u| u.origin().ascii_serialization())
}

fn fields(interface: &WebInterface) -> Value {
    json!({
        "interfaceId": interface.id.to_string(),
        "origin": origin(&interface.url),
        "label": interface.label,
        "openIn": interface.open_in.as_str(),
    })
}

/// The audit entry is on the device (D-3); the event is about the same device (CR-5).
fn entry(device: &Device, action: AuditAction, message: String, change: Value) -> TrailEntry {
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message,
        action,
        target: AuditTarget::Device(device.id),
        change,
    }
}

pub(super) fn added(interface: &WebInterface, device: &Device) -> TrailEntry {
    entry(
        device,
        AuditAction::Create,
        format!("Web interface {} added.", interface.label),
        fields(interface),
    )
}

/// `previousOrigin` and `previousLabel` appear when they changed. `pinDropped` is set when the new origin removed the certificate the operator accepted.
pub(super) fn updated(before: &WebInterface, after: &WebInterface, device: &Device) -> TrailEntry {
    let mut change = fields(after);
    if before.pin.is_some() && after.pin.is_none() {
        change["pinDropped"] = json!(true);
    }
    if origin(&before.url) != origin(&after.url) {
        change["previousOrigin"] = json!(origin(&before.url));
    }
    if before.label != after.label {
        change["previousLabel"] = json!(before.label);
    }
    entry(
        device,
        AuditAction::Update,
        format!("Web interface {} updated.", after.label),
        change,
    )
}

pub(super) fn deleted(id: WebInterfaceId, device: &Device) -> TrailEntry {
    entry(
        device,
        AuditAction::Delete,
        "Web interface deleted.".to_owned(),
        json!({ "interfaceId": id.to_string() }),
    )
}

/// `created` is false when the device already had an interface at that URL.
pub(super) fn suggestion_accepted(
    interface: &WebInterface,
    device: &Device,
    (check, created): (CheckId, bool),
) -> TrailEntry {
    let mut change = fields(interface);
    change["checkId"] = json!(check.to_string());
    change["created"] = json!(created);
    entry(
        device,
        AuditAction::Accept,
        format!("Web interface {} accepted from a check.", interface.label),
        change,
    )
}

/// Hiding a suggestion changes what the device offers and removes nothing, so it is an
/// `update` of the device.
pub(super) fn suggestion_dismissed(check: CheckId, device: &Device) -> TrailEntry {
    entry(
        device,
        AuditAction::Update,
        "Web interface suggestion dismissed.".to_owned(),
        json!({ "checkId": check.to_string(), "dismissed": true }),
    )
}

/// The fingerprint is public (WEB-009).
pub(super) fn certificate_accepted(interface: &WebInterface, device: &Device) -> TrailEntry {
    let mut change = fields(interface);
    if let Some(pin) = &interface.pin {
        change["fingerprint"] = json!(pin.fingerprint.hex());
    }
    entry(
        device,
        AuditAction::Accept,
        format!("Certificate of web interface {} accepted.", interface.label),
        change,
    )
}
