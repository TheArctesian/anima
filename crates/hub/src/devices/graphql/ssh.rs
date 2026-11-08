//! A device's SSH settings as the GraphQL type K1 calls `SshSettings`, with whether the hub
//! holds a secret for it (DEV-012, SEC-005, API-005) and the host key accepted for it
//! (SEC-006; the R-011 composition edge to `trust`).

use std::{collections::HashMap, sync::Arc};

use async_graphql::{
    Context, Object, Result,
    dataloader::{DataLoader, Loader},
};
use sqlx::PgPool;

use crate::{
    api_errors::{internal, schema_dependency},
    credentials::{self, with_ssh},
    devices::{SshAuth, model::SshSettings},
    ids::DeviceId,
    trust::{HostKey, graphql::HostKeysByDevice},
};

/// The SSH settings of one device. `credential_stored` is set when the resolver that built
/// it already knows the answer, so `credentialStored` costs no query.
pub(super) struct DeviceSsh {
    pub(super) device: DeviceId,
    pub(super) settings: SshSettings,
    pub(super) credential_stored: Option<bool>,
}

#[Object(name = "SshSettings")]
impl DeviceSsh {
    async fn enabled(&self) -> bool {
        self.settings.enabled
    }

    async fn port(&self) -> i32 {
        i32::from(self.settings.port)
    }

    async fn username(&self) -> &str {
        &self.settings.username
    }

    async fn auth(&self) -> SshAuth {
        self.settings.auth
    }

    /// True when the hub holds a secret for this device. Secrets are never returned.
    async fn credential_stored(&self, ctx: &Context<'_>) -> Result<bool> {
        if let Some(known) = self.credential_stored {
            return Ok(known);
        }
        let stored = schema_dependency::<DataLoader<CredentialStored>>(ctx)?
            .load_one(self.device)
            .await
            .map_err(internal)?;
        Ok(stored.unwrap_or(false))
    }

    /// The host key the operator accepted for this device, if any (SEC-006).
    async fn host_key(&self, ctx: &Context<'_>) -> Result<Option<HostKey>> {
        schema_dependency::<DataLoader<HostKeysByDevice>>(ctx)?
            .load_one(self.device)
            .await
            .map_err(internal)
    }
}

/// Whether each device has an SSH secret, for `SshSettings.credentialStored`: one query per
/// batch of device ids.
pub struct CredentialStored(pub(super) PgPool);

impl Loader<DeviceId> for CredentialStored {
    type Value = bool;
    type Error = Arc<credentials::Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, bool>, Self::Error> {
        let stored = with_ssh(&self.0, keys).await.map_err(Arc::new)?;
        Ok(keys
            .iter()
            .map(|key| (*key, stored.contains(key)))
            .collect())
    }
}
