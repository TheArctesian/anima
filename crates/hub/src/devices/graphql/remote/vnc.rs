//! A device's VNC settings as the GraphQL type K1 calls `VncSettings`, with whether the hub
//! holds a password for it (RA-030, DEV-011, SEC-005, API-005).

use std::{collections::HashMap, sync::Arc};

use async_graphql::{Context, Object, Result, dataloader::Loader};
use sqlx::PgPool;

use super::{credential_stored, passwords_stored};
use crate::{
    credentials::{self, PasswordKind},
    devices::{Device, VncSettings},
    ids::DeviceId,
};

/// The VNC settings of one device. `credential_stored` is set when the resolver that built it
/// already knows the answer, so `credentialStored` costs no query.
pub(in crate::devices::graphql) struct DeviceVnc {
    device: DeviceId,
    settings: VncSettings,
    credential_stored: Option<bool>,
}

impl DeviceVnc {
    pub(in crate::devices::graphql) fn new(
        device: &Device,
        credential_stored: Option<bool>,
    ) -> Self {
        Self {
            device: device.id,
            settings: device.vnc.clone(),
            credential_stored,
        }
    }
}

#[Object(name = "VncSettings")]
impl DeviceVnc {
    async fn enabled(&self) -> bool {
        self.settings.enabled
    }

    async fn port(&self) -> i32 {
        i32::from(self.settings.port)
    }

    /// macOS Screen Sharing needs one.
    async fn username(&self) -> Option<&str> {
        self.settings.username.as_deref()
    }

    /// True when the hub holds a VNC password for this device. Passwords are never returned.
    async fn credential_stored(&self, ctx: &Context<'_>) -> Result<bool> {
        credential_stored::<VncCredentialStored>(ctx, self.device, self.credential_stored).await
    }
}

/// Whether each device has a VNC password, for `VncSettings.credentialStored`: one query per
/// batch of device ids.
pub struct VncCredentialStored(pub(in crate::devices::graphql) PgPool);

impl Loader<DeviceId> for VncCredentialStored {
    type Value = bool;
    type Error = Arc<credentials::Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, bool>, Self::Error> {
        passwords_stored(&self.0, keys, PasswordKind::Vnc).await
    }
}
