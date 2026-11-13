//! A device's RDP settings as the GraphQL type K1 calls `RdpSettings`, with whether the hub
//! holds a password for it (RA-030, DEV-011, SEC-005, API-005) and the server certificate
//! accepted for it (SEC-007).

use std::{collections::HashMap, sync::Arc};

use async_graphql::{
    Context, Object, Result,
    dataloader::{DataLoader, Loader},
};
use sqlx::PgPool;

use super::{credential_stored, passwords_stored};
use crate::{
    api_errors::{internal, schema_dependency},
    fleet::{
        credentials::{self, PasswordKind},
        devices::{Device, RdpSecurity, RdpSettings, ResizeMethod},
        trust::{ServerCertificate, graphql::ServerCertificatesByDevice},
    },
    ids::DeviceId,
};

/// The RDP settings of one device. `credential_stored` is set when the resolver that built it
/// already knows the answer, so `credentialStored` costs no query.
pub(in crate::fleet::devices::graphql) struct DeviceRdp {
    device: DeviceId,
    settings: RdpSettings,
    credential_stored: Option<bool>,
}

impl DeviceRdp {
    pub(in crate::fleet::devices::graphql) fn new(
        device: &Device,
        credential_stored: Option<bool>,
    ) -> Self {
        Self {
            device: device.id,
            settings: device.rdp.clone(),
            credential_stored,
        }
    }
}

#[Object(name = "RdpSettings")]
impl DeviceRdp {
    async fn enabled(&self) -> bool {
        self.settings.enabled
    }

    async fn port(&self) -> i32 {
        i32::from(self.settings.port)
    }

    async fn username(&self) -> &str {
        &self.settings.username
    }

    async fn domain(&self) -> Option<&str> {
        self.settings.domain.as_deref()
    }

    async fn security(&self) -> RdpSecurity {
        self.settings.security
    }

    async fn ignore_certificate(&self) -> bool {
        self.settings.ignore_certificate
    }

    async fn resize_method(&self) -> ResizeMethod {
        self.settings.resize
    }

    async fn audio_out(&self) -> bool {
        self.settings.audio_out
    }

    async fn microphone(&self) -> bool {
        self.settings.microphone
    }

    async fn shared_drive(&self) -> bool {
        self.settings.shared_drive
    }

    /// True when the hub holds an RDP password for this device. Passwords are never returned.
    async fn credential_stored(&self, ctx: &Context<'_>) -> Result<bool> {
        credential_stored::<RdpCredentialStored>(ctx, self.device, self.credential_stored).await
    }

    /// The server certificate the operator accepted for this device, if any (SEC-007).
    async fn server_certificate(&self, ctx: &Context<'_>) -> Result<Option<ServerCertificate>> {
        schema_dependency::<DataLoader<ServerCertificatesByDevice>>(ctx)?
            .load_one(self.device)
            .await
            .map_err(internal)
    }
}

/// Whether each device has an RDP password, for `RdpSettings.credentialStored`: one query per
/// batch of device ids.
pub struct RdpCredentialStored(pub(in crate::fleet::devices::graphql) PgPool);

impl Loader<DeviceId> for RdpCredentialStored {
    type Value = bool;
    type Error = Arc<credentials::Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, bool>, Self::Error> {
        passwords_stored(&self.0, keys, PasswordKind::Rdp).await
    }
}
