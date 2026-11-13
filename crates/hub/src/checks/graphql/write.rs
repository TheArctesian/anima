//! The four check mutations. Each writes its change, one audit entry and one event in one
//! transaction: if any step fails, nothing is stored (API-003, LOG-005, CHK-005). A new check
//! takes the default notification channels in the same transaction (NOT-003).

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
    checks::{self, Check, CheckChange, FieldError, Sealer, config::CheckType, store},
    credentials::Vault,
    devices::{self, Device},
    events::Trail,
    ids::{CheckId, DeviceId},
    notifications,
};

/// The device a new check goes on; a missing one is the operator's `deviceId` error.
async fn new_device(db: &PgPool, device: DeviceId) -> Result<Device> {
    match devices::get(db, device).await {
        Ok(found) => Ok(found),
        Err(devices::Error::NotFound(_)) => Err(invalid(&[FieldError::new(
            "deviceId",
            "There is no such device.",
        )])),
        Err(error) => Err(from_devices(error)),
    }
}

/// Every device is read before the transaction opens: a mutation never holds one connection
/// while it waits for a second (API-010). Its host as read here is the host SNMP secrets are
/// sealed for; if it changes before the commit, the probe finds the secrets bound to another
/// host and sends nothing (SEC-010).
async fn device_of(db: &PgPool, device: DeviceId) -> Result<Device> {
    devices::get(db, device).await.map_err(from_devices)
}

/// The device's name, for the audit entry.
async fn device_name(db: &PgPool, device: DeviceId) -> Result<String> {
    device_of(db, device).await.map(|device| device.name)
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
    sealer: Sealer<'_>,
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
        checks::update_check(conn, sealer, child, change)
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
    let device = new_device(db, form.new.device).await?;
    let sealer = Sealer {
        vault: schema_dependency::<Vault>(ctx)?,
        host: &device.host,
    };
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let check = checks::create_check(&mut tx, sealer, form.new)
        .await
        .map_err(to_graphql)?;
    // A new check notifies the channels marked "on by default" (NOT-003).
    notifications::adopt_defaults(&mut tx, check.id)
        .await
        .map_err(internal)?;
    if let Some(children) = &form.children {
        regroup(&mut tx, sealer, check.id, (&[], children)).await?;
    }
    let extras = Extras {
        sealed: form.sealed,
        children: form.children.as_deref(),
        rotated: false,
    };
    trail
        .write(&mut tx, trail::created(&check, &device.name, &extras))
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
    let device = device_of(db, before.device).await?;
    if let Some(children) = &form.children {
        validate_children(db, children, before.device).await?;
    }
    let sealer = Sealer {
        vault: schema_dependency::<Vault>(ctx)?,
        host: &device.host,
    };
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    // The audit compares against the row as it is under our lock, not as it was when the
    // request arrived (LOG-005).
    let before = store::fetch_for_update(&mut tx, id)
        .await
        .map_err(to_graphql)?;
    let after = checks::update_check(&mut tx, sealer, id, form.change)
        .await
        .map_err(to_graphql)?;
    if let Some(children) = &form.children {
        let current = store::children_for_update(&mut tx, id)
            .await
            .map_err(to_graphql)?;
        regroup(&mut tx, sealer, id, (&current, children)).await?;
    }
    let extras = Extras {
        sealed: form.sealed,
        children: form.children.as_deref(),
        rotated: form.rotated,
    };
    trail
        .write(
            &mut tx,
            trail::updated(&before, &after, &device.name, &extras),
        )
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
