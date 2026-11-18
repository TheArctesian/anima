//! The audit entry and event of each place change (LOG-004, LOG-005). Pure. Neither holds a
//! position: a place's centre is where the operator lives or works.

use serde_json::{Value, json};

use super::model::{Divided, Merged, Moved, Place, Removal, Siting};
use crate::{
    fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
    ids::PlaceId,
};

fn entry(target: PlaceId, action: AuditAction, message: String, change: Value) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target: AuditTarget::Place(target),
        change,
    }
}

fn moved(moved: &Moved) -> Value {
    json!({
        "links": moved.links,
        "sites": moved.sites,
        "children": moved.children,
        "held": moved.held.iter().map(|(name, n)| json!({ "domain": name, "count": n })).collect::<Vec<_>>(),
    })
}

pub(super) fn created(place: &Place) -> TrailEntry {
    entry(
        place.id,
        AuditAction::Create,
        format!("Place added: {}.", place.name),
        json!({ "name": place.name, "category": place.category.as_str(), "parent": place.parent.map(|p| p.to_string()) }),
    )
}

pub(super) fn updated(place: &Place, changed: &[&str]) -> TrailEntry {
    entry(
        place.id,
        AuditAction::Update,
        format!("Place changed: {}.", place.name),
        json!({ "name": place.name, "changed": changed }),
    )
}

pub(super) fn merged(merged: &Merged, action: AuditAction) -> TrailEntry {
    entry(
        merged.from.id,
        action,
        format!(
            "Place merged: {} into {}.",
            merged.from.name, merged.into.name
        ),
        json!({
            "name": merged.from.name,
            "mergedInto": merged.into.id.to_string(),
            "moved": moved(&merged.moved),
        }),
    )
}

pub(super) fn divided(divided: &Divided) -> TrailEntry {
    let name = &divided.from.name;
    entry(
        divided.from.id,
        AuditAction::Update,
        format!("Place split: {name} into {}.", divided.new.name),
        json!({
            "name": name,
            "new": divided.new.id.to_string(),
            "moved": moved(&divided.moved),
        }),
    )
}

pub(super) fn removed(removal: &Removal) -> TrailEntry {
    match removal {
        Removal::Deleted(place) => entry(
            place.id,
            AuditAction::Delete,
            format!("Place removed: {}.", place.name),
            json!({ "name": place.name }),
        ),
        Removal::Replaced(done) => merged(done, AuditAction::Delete),
    }
}

/// The entry of a site change; `None` when the site already had this place.
pub(super) fn sited(siting: &Siting) -> Option<TrailEntry> {
    let target = siting.after.or(siting.before)?;
    (siting.before != siting.after).then(|| {
        entry(
            target,
            AuditAction::Update,
            format!(
                "Site {} {}.",
                siting.site,
                if siting.after.is_some() {
                    "placed"
                } else {
                    "no longer placed"
                }
            ),
            json!({
                "site": siting.site.to_string(),
                "before": siting.before.map(|p| p.to_string()),
                "after": siting.after.map(|p| p.to_string()),
            }),
        )
    })
}
