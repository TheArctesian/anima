//! `setRdpCredential`, `clearRdpCredential`, `setVncCredential` and `clearVncCredential`: the
//! password goes to `credentials`, sealed for its device and kind, and the audit entry and the
//! event name only the protocol and "password" (DEV-011, SEC-003, SEC-005, LOG-005, LOG-007).
//!
//! Each runs in one transaction. `credentials` takes its per-device advisory lock, the one lock
//! whose order `credentials` asks of its callers; the device read takes no row lock. A set
//! reads the device first, to bind the password to the device's host now (SEC-011). The SSH
//! credential works the same way.

use async_graphql::{Context, ID, Result};

use crate::{
    api_errors::{begin, commit, internal, invalid, schema_dependency},
    fleet::{
        credentials::{PasswordKind, Vault, clear_password as clear, store_password},
        devices::{
            Device,
            graphql::{
                credential::{credential_error, device_arg},
                credential_input::PasswordCredential,
                error::to_graphql,
                input::checked,
                trail,
            },
            store,
        },
        events::Trail,
    },
};

/// Replaces the device's password of `kind`, then audits and records the change. Returns the
/// device as read inside the same transaction.
pub(in crate::fleet::devices::graphql) async fn set_password(
    ctx: &Context<'_>,
    device: &ID,
    credential: PasswordCredential,
    kind: PasswordKind,
) -> Result<Device> {
    let id = device_arg(device);
    let secret = checked(credential.into_checked(), id.as_ref().err().cloned())?;
    let id = id.map_err(|error| invalid(&[error]))?;
    let vault = schema_dependency::<Vault>(ctx)?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let device = store::fetch(&mut *tx, id).await.map_err(to_graphql)?;
    store_password(&mut tx, vault, (id, &device.host), (kind, secret))
        .await
        .map_err(|error| credential_error(error, id))?;
    trail
        .write(&mut tx, trail::password_set(&device, kind))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(device)
}

/// Removes the device's password of `kind`. Only a removal is audited and recorded: clearing a
/// device that has no such password changes nothing.
pub(in crate::fleet::devices::graphql) async fn clear_password(
    ctx: &Context<'_>,
    device: &ID,
    kind: PasswordKind,
) -> Result<Device> {
    let id = device_arg(device).map_err(|error| invalid(&[error]))?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let removed = clear(&mut tx, id, kind)
        .await
        .map_err(|error| credential_error(error, id))?;
    let device = store::fetch(&mut *tx, id).await.map_err(to_graphql)?;
    if removed {
        trail
            .write(&mut tx, trail::password_cleared(&device, kind))
            .await
            .map_err(internal)?;
    }
    commit(tx).await?;
    Ok(device)
}
