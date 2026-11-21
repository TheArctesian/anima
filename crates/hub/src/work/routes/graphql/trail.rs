//! What a route mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005). The audit target is the device of the route.
//! A `change` names the route, the name, the application and the port; never the token of the
//! proof (LOG-007).

use serde_json::{Value, json};

use crate::{
    fleet::{
        devices::Device,
        events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    },
    ids::RouteId,
    work::routes::{Route, proof::Proof},
};

fn fields(route: &Route) -> Value {
    json!({
        "routeId": route.id.to_string(),
        "name": route.name.as_str(),
        "applicationId": route.application.to_string(),
        "port": route.port,
    })
}

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

pub(super) fn added(route: &Route, device: &Device) -> TrailEntry {
    entry(
        device,
        AuditAction::Create,
        format!("Public name {} added; it waits for its proof.", route.name),
        fields(route),
    )
}

pub(super) fn checked(route: &Route, device: &Device) -> TrailEntry {
    let state = match route.proof {
        Proof::Waiting => "not proven yet",
        Proof::Proven => "proven",
        Proof::Lost => "lost",
    };
    let mut change = fields(route);
    change["proof"] = json!(state);
    change["failedReads"] = json!(route.failed_reads);
    entry(
        device,
        AuditAction::Update,
        format!("Public name {} checked: {state}.", route.name),
        change,
    )
}

pub(super) fn removed(route: RouteId, name: &str, device: &Device) -> TrailEntry {
    entry(
        device,
        AuditAction::Delete,
        format!("Public name {name} removed."),
        json!({ "routeId": route.to_string(), "name": name }),
    )
}
