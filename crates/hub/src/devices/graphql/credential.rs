//! `setSshCredential` and `clearSshCredential`: the secret goes to `credentials`, sealed, and
//! the audit entry and the event say only what kind of secret changed (DEV-012, SEC-003,
//! SEC-005, LOG-005, LOG-007).
//!
//! Each runs in one transaction. `credentials` takes its per-device lock first, before this
//! domain reads the device row, the lock order `credentials` asks of its callers.

use async_graphql::{Context, Error as GraphQlError, ID, Result};

use super::{
    credential_input::{SshCredential, checked_secret},
    error::to_graphql,
    input::checked,
    ssh::DeviceSsh,
    trail,
};
use crate::{
    api_errors::{begin, commit, internal, invalid, schema_dependency},
    credentials::{Vault, clear_ssh, store_ssh},
    devices::{Device, Error, FieldError, Missing, store},
    events::Trail,
    ids::DeviceId,
};

pub(super) fn device_arg(id: &ID) -> Result<DeviceId, FieldError> {
    id.parse()
        .map_err(|_| FieldError::new("deviceId", "That device id is not valid."))
}

/// `UnknownDevice` is the operator's `NOT_FOUND`; any other failure is `INTERNAL` with its
/// cause kept as the hidden source. No `credentials` error message holds a secret.
pub(super) fn credential_error(error: crate::credentials::Error, device: DeviceId) -> GraphQlError {
    match error {
        crate::credentials::Error::UnknownDevice => {
            to_graphql(Error::NotFound(Missing::Device(device)))
        }
        other => internal(other),
    }
}

fn settings(device: Device, credential_stored: bool) -> DeviceSsh {
    DeviceSsh {
        device: device.id,
        settings: device.ssh,
        credential_stored: Some(credential_stored),
    }
}

/// Replaces the device's SSH secret, then audits and records the change.
pub(super) async fn set(
    ctx: &Context<'_>,
    device: &ID,
    credential: SshCredential,
) -> Result<DeviceSsh> {
    let SshCredential(secret) = credential;
    let id = device_arg(device);
    let kind = checked(checked_secret(&secret), id.as_ref().err().cloned())?;
    let id = id.map_err(|error| invalid(&[error]))?;
    let vault = schema_dependency::<Vault>(ctx)?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    store_ssh(&mut tx, vault, id, secret)
        .await
        .map_err(|error| credential_error(error, id))?;
    let device = store::fetch(&mut *tx, id).await.map_err(to_graphql)?;
    trail
        .write(&mut tx, trail::credential_set(&device, kind))
        .await
        .map_err(internal)?;
    commit(tx).await?;
    Ok(settings(device, true))
}

/// Removes the device's SSH secret. Only a removal is audited and recorded: clearing a device
/// that has no secret changes nothing.
pub(super) async fn clear(ctx: &Context<'_>, device: &ID) -> Result<DeviceSsh> {
    let id = device_arg(device).map_err(|error| invalid(&[error]))?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let removed = clear_ssh(&mut tx, id)
        .await
        .map_err(|error| credential_error(error, id))?;
    let device = store::fetch(&mut *tx, id).await.map_err(to_graphql)?;
    if removed {
        trail
            .write(&mut tx, trail::credential_cleared(&device))
            .await
            .map_err(internal)?;
    }
    commit(tx).await?;
    Ok(settings(device, false))
}
