//! The four check mutations. Each writes its change, one audit entry and one event in one
//! transaction: if any step fails, nothing is stored (API-003, LOG-005, CHK-005).

use async_graphql::{Context, ID, Result};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

use super::{
    error::{from_devices, invalid, schema_dependency, to_graphql},
    form::{self, check_id},
    input::{CreateShape, UpdateShape},
    trail::{self, Extras, Trail},
};
use crate::{
    checks::{self, Check, CheckChange, Error, FieldError, PushToken, config::CheckType},
    credentials::Vault,
    devices,
    ids::{CheckId, DeviceId},
};

pub(super) fn pool<'a>(ctx: &Context<'a>) -> Result<&'a PgPool> {
    schema_dependency::<PgPool>(ctx)
}

/// The transaction a mutation writes its change, its audit entry and its event in.
async fn begin(ctx: &Context<'_>) -> Result<Transaction<'static, Postgres>> {
    pool(ctx)?
        .begin()
        .await
        .map_err(|e| to_graphql(Error::Database(e)))
}

async fn commit(tx: Transaction<'static, Postgres>) -> Result<()> {
    tx.commit()
        .await
        .map_err(|e| to_graphql(Error::Database(e)))
}

/// A new push token from 32 random bytes of the operating system's generator: the I/O edge
/// of token making (R-028).
fn fresh_token() -> PushToken {
    let mut bytes = [0_u8; 32];
    rand::fill(&mut bytes);
    PushToken::from_random(&bytes)
}

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

/// Every child must exist, be on the group's device and not be a group.
async fn validate_children(db: &PgPool, children: &[CheckId], device: DeviceId) -> Result<()> {
    let mut problems = Vec::new();
    for &child in children {
        match checks::get(db, child).await {
            Ok(found) => problems.extend(
                child_problem(&found, device).map(|problem| FieldError::new("childIds", problem)),
            ),
            Err(Error::NotFound(_)) => {
                problems.push(FieldError::new("childIds", "A child check does not exist."));
            }
            Err(error) => return Err(to_graphql(error)),
        }
    }
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
            ..CheckChange::default()
        };
        checks::update_check(conn, vault, child, change)
            .await
            .map_err(to_graphql)?;
    }
    Ok(())
}

pub(super) async fn create(ctx: &Context<'_>, input: CreateShape) -> Result<Check> {
    let form = form::create(input, fresh_token()).map_err(|errors| invalid(&errors))?;
    let db = pool(ctx)?;
    if let Some(children) = &form.children {
        validate_children(db, children, form.new.device).await?;
    }
    let vault = schema_dependency::<Vault>(ctx)?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let check = checks::create_check(&mut tx, vault, form.new)
        .await
        .map_err(to_graphql)?;
    if let Some(children) = &form.children {
        regroup(&mut tx, vault, check.id, (&[], children)).await?;
    }
    let device = device_name(db, check.device).await?;
    let extras = Extras {
        sealed: form.sealed,
        children: form.children.as_deref(),
        rotated: false,
    };
    trail
        .write(&mut tx, trail::created(&check, &device, &extras))
        .await?;
    commit(tx).await?;
    Ok(check)
}

pub(super) async fn update(ctx: &Context<'_>, id: &ID, input: UpdateShape) -> Result<Check> {
    let id = check_id(id).map_err(|error| invalid(&[error]))?;
    let db = pool(ctx)?;
    let before = checks::get(db, id).await.map_err(to_graphql)?;
    let form = form::update(input, fresh_token()).map_err(|errors| invalid(&errors))?;
    let current = match &form.children {
        Some(children) => {
            validate_children(db, children, before.device).await?;
            let siblings = checks::for_device(db, before.device)
                .await
                .map_err(to_graphql)?;
            let members = siblings.iter().filter(|check| check.parent == Some(id));
            members.map(|check| check.id).collect()
        }
        None => Vec::new(),
    };
    let vault = schema_dependency::<Vault>(ctx)?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let after = checks::update_check(&mut tx, vault, id, form.change)
        .await
        .map_err(to_graphql)?;
    if let Some(children) = &form.children {
        regroup(&mut tx, vault, id, (&current, children)).await?;
    }
    let device = device_name(db, after.device).await?;
    let extras = Extras {
        sealed: form.sealed,
        children: form.children.as_deref(),
        rotated: form.rotated,
    };
    trail
        .write(&mut tx, trail::updated(&before, &after, &device, &extras))
        .await?;
    commit(tx).await?;
    Ok(after)
}

pub(super) async fn delete(ctx: &Context<'_>, id: &ID) -> Result<ID> {
    let id = check_id(id).map_err(|error| invalid(&[error]))?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let check = checks::delete_check(&mut tx, id)
        .await
        .map_err(to_graphql)?;
    let device = device_name(pool(ctx)?, check.device).await?;
    trail
        .write(&mut tx, trail::deleted(&check, &device))
        .await?;
    commit(tx).await?;
    Ok(ID(check.id.to_string()))
}

pub(super) async fn set_paused(ctx: &Context<'_>, id: &ID, paused: bool) -> Result<Check> {
    let id = check_id(id).map_err(|error| invalid(&[error]))?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let check = checks::set_paused(&mut tx, id, paused)
        .await
        .map_err(to_graphql)?;
    let device = device_name(pool(ctx)?, check.device).await?;
    trail.write(&mut tx, trail::paused(&check, &device)).await?;
    commit(tx).await?;
    Ok(check)
}
