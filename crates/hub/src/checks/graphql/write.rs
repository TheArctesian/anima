//! The four check mutations. Each writes its change, one audit entry and one event in one
//! transaction: if any step fails, nothing is stored (API-003, LOG-005, CHK-005).

use std::collections::HashMap;

use async_graphql::{Context, ID, Result};
use sqlx::{PgConnection, PgPool};

use super::{
    error::{from_devices, to_graphql},
    form::{self, check_id},
    input::{CreateShape, UpdateShape},
    trail::{self, Extras},
};
use crate::{
    api_errors::{begin, commit, internal, invalid, pool, schema_dependency},
    checks::{self, Check, CheckChange, FieldError, config::CheckType, store},
    credentials::Vault,
    devices,
    events::Trail,
    ids::{CheckId, DeviceId},
};

/// The device a new check goes on; a missing one is the operator's `deviceId` error.
async fn new_device_name(db: &PgPool, device: DeviceId) -> Result<String> {
    match devices::get(db, device).await {
        Ok(found) => Ok(found.name),
        Err(devices::Error::NotFound(_)) => Err(invalid(&[FieldError::new(
            "deviceId",
            "There is no such device.",
        )])),
        Err(error) => Err(from_devices(error)),
    }
}

/// Every device name is read before the transaction opens: a mutation never holds one
/// connection while it waits for a second (API-010).
async fn device_name(db: &PgPool, device: DeviceId) -> Result<String> {
    devices::get(db, device)
        .await
        .map(|device| device.name)
        .map_err(from_devices)
}

/// Why this check cannot be a child of a group on `device`.
fn child_problem(child: &Check, device: DeviceId) -> Option<&'static str> {
    if child.device != device {
        Some("A child must be on the same device as the group.")
    } else if child.kind() == CheckType::Group {
        Some("A group cannot be a child of a group.")
    } else {
        None
    }
}

/// Every child must exist, be on the group's device and not be a group. One query reads them
/// all (API-005).
async fn validate_children(db: &PgPool, children: &[CheckId], device: DeviceId) -> Result<()> {
    let found: HashMap<CheckId, Check> = checks::get_many(db, children)
        .await
        .map_err(to_graphql)?
        .into_iter()
        .map(|check| (check.id, check))
        .collect();
    let problems: Vec<FieldError> = children
        .iter()
        .filter_map(|child| match found.get(child) {
            Some(check) => child_problem(check, device),
            None => Some("A child check does not exist."),
        })
        .map(|problem| FieldError::new("childIds", problem))
        .collect();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(invalid(&problems))
    }
}

/// Makes `wanted` the children of `group`, and lets out those of `current` not in `wanted`.
async fn regroup(
    conn: &mut PgConnection,
    vault: &Vault,
    group: CheckId,
    (current, wanted): (&[CheckId], &[CheckId]),
) -> Result<()> {
    let leaving = current.iter().filter(|child| !wanted.contains(child));
    let moves = leaving
        .map(|child| (*child, None))
        .chain(wanted.iter().map(|child| (*child, Some(group))));
    for (child, parent) in moves {
        let change = CheckChange {
            parent: Some(parent),
            regrouping: true,
            ..CheckChange::default()
        };
        checks::update_check(conn, vault, child, change)
            .await
            .map_err(to_graphql)?;
    }
    Ok(())
}

pub(super) async fn create(ctx: &Context<'_>, input: CreateShape) -> Result<Check> {
    let form = form::create(input, checks::fresh_push_token).map_err(to_graphql)?;
    let db = pool(ctx)?;
    if let Some(children) = &form.children {
        validate_children(db, children, form.new.device).await?;
    }
    let device = new_device_name(db, form.new.device).await?;
    let vault = schema_dependency::<Vault>(ctx)?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let check = checks::create_check(&mut tx, vault, form.new)
        .await
        .map_err(to_graphql)?;
    if let Some(children) = &form.children {
        regroup(&mut tx, vault, check.id, (&[], children)).await?;
    }
    let extras = Extras {
        sealed: form.sealed,
        children: form.children.as_deref(),
        rotated: false,
    };
    trail
        .write(&mut tx, trail::created(&check, &device, &extras))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(check)
}

pub(super) async fn update(ctx: &Context<'_>, id: &ID, input: UpdateShape) -> Result<Check> {
    let id = check_id(id).map_err(|error| invalid(&[error]))?;
    let db = pool(ctx)?;
    let before = checks::get(db, id).await.map_err(to_graphql)?;
    // A push configuration is never empty, so its token is drawn only for a real change.
    let form = form::update(input, checks::fresh_push_token).map_err(to_graphql)?;
    if form.is_empty() {
        let nothing = FieldError::new("input", "Send at least one field to change.");
        return Err(invalid(&[nothing]));
    }
    let device = device_name(db, before.device).await?;
    if let Some(children) = &form.children {
        validate_children(db, children, before.device).await?;
    }
    let vault = schema_dependency::<Vault>(ctx)?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    // The audit compares against the row as it is under our lock, not as it was when the
    // request arrived (LOG-005).
    let before = store::fetch_for_update(&mut tx, id)
        .await
        .map_err(to_graphql)?;
    let after = checks::update_check(&mut tx, vault, id, form.change)
        .await
        .map_err(to_graphql)?;
    if let Some(children) = &form.children {
        let current = store::children_for_update(&mut tx, id)
            .await
            .map_err(to_graphql)?;
        regroup(&mut tx, vault, id, (&current, children)).await?;
    }
    let extras = Extras {
        sealed: form.sealed,
        children: form.children.as_deref(),
        rotated: form.rotated,
    };
    trail
        .write(&mut tx, trail::updated(&before, &after, &device, &extras))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(after)
}

pub(super) async fn delete(ctx: &Context<'_>, id: &ID) -> Result<ID> {
    let id = check_id(id).map_err(|error| invalid(&[error]))?;
    let db = pool(ctx)?;
    let before = checks::get(db, id).await.map_err(to_graphql)?;
    let device = device_name(db, before.device).await?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    // The group is locked first, then its children, so the audit lists exactly those it frees.
    let locked = store::fetch_for_update(&mut tx, id)
        .await
        .map_err(to_graphql)?;
    let freed = if locked.kind() == CheckType::Group {
        store::children_for_update(&mut tx, id)
            .await
            .map_err(to_graphql)?
    } else {
        Vec::new()
    };
    let check = checks::delete_check(&mut tx, id)
        .await
        .map_err(to_graphql)?;
    trail
        .write(&mut tx, trail::deleted(&check, &device, &freed))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(ID(check.id.to_string()))
}

pub(super) async fn set_paused(ctx: &Context<'_>, id: &ID, paused: bool) -> Result<Check> {
    let id = check_id(id).map_err(|error| invalid(&[error]))?;
    let db = pool(ctx)?;
    let before = checks::get(db, id).await.map_err(to_graphql)?;
    let device = device_name(db, before.device).await?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let check = checks::set_paused(&mut tx, id, paused)
        .await
        .map_err(to_graphql)?;
    trail
        .write(&mut tx, trail::paused(&check, &device))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(check)
}
