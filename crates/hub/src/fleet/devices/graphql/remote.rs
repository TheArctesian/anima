//! The remote access part of the devices GraphQL (K1): the RDP, VNC and transfer settings of a
//! device, whether a password is stored for each protocol, the password mutations and the
//! site's wake relay (RA-020, RA-030..033, DEV-011, SEC-005, D-7).

mod input;
mod password;
mod rdp;
mod relay;
mod vnc;

use std::{collections::HashMap, sync::Arc};

use async_graphql::{
    Context, Object, Result,
    dataloader::{DataLoader, Loader},
};
use sqlx::PgPool;

pub use self::{
    input::{RdpSettingsInput, TransferPolicyInput, VncSettingsInput},
    rdp::RdpCredentialStored,
    vnc::VncCredentialStored,
};
pub(super) use self::{
    password::{clear_password, set_password},
    rdp::DeviceRdp,
    relay::set_wake_relay,
    vnc::DeviceVnc,
};

use crate::{
    api_errors::{internal, schema_dependency},
    fleet::{
        credentials::{self, PasswordKind, with_password},
        devices::TransferPolicy,
    },
    ids::DeviceId,
};

#[Object]
impl TransferPolicy {
    async fn upload(&self) -> bool {
        self.upload
    }

    async fn download(&self) -> bool {
        self.download
    }

    async fn copy(&self) -> bool {
        self.copy
    }

    async fn paste(&self) -> bool {
        self.paste
    }
}

/// Whether each device has a password of `kind`, for the RDP and VNC `credentialStored`
/// loaders: one query per batch.
async fn passwords_stored(
    db: &PgPool,
    keys: &[DeviceId],
    kind: PasswordKind,
) -> Result<HashMap<DeviceId, bool>, Arc<credentials::Error>> {
    let stored = with_password(db, keys, kind).await.map_err(Arc::new)?;
    Ok(keys
        .iter()
        .map(|key| (*key, stored.contains(key)))
        .collect())
}

/// `credentialStored` for one device: the answer the resolver that built the settings already
/// knows, else the loader's.
async fn credential_stored<L>(
    ctx: &Context<'_>,
    device: DeviceId,
    known: Option<bool>,
) -> Result<bool>
where
    L: Loader<DeviceId, Value = bool, Error = Arc<credentials::Error>>,
{
    if let Some(known) = known {
        return Ok(known);
    }
    let stored = schema_dependency::<DataLoader<L>>(ctx)?
        .load_one(device)
        .await
        .map_err(internal)?;
    Ok(stored.unwrap_or(false))
}
