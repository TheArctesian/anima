//! `ServerCertificate` as the GraphQL type K1 describes, and the loader behind
//! `RdpSettings.serverCertificate` (SEC-007, API-005).

use std::{collections::HashMap, sync::Arc};

use async_graphql::{Object, dataloader::Loader};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    ids::DeviceId,
    trust::{Error, ServerCertificate, server_certificates},
};

#[Object]
impl ServerCertificate {
    /// SHA-256 of the leaf certificate's DER, as 32 lower-case hex pairs joined by colons.
    async fn fingerprint_sha256(&self) -> &str {
        &self.fingerprint_sha256
    }

    /// The certificate's subject, as the device chose it.
    async fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    async fn not_after(&self) -> Option<DateTime<Utc>> {
        self.not_after
    }

    async fn first_seen_at(&self) -> DateTime<Utc> {
        self.first_seen_at
    }
}

/// The accepted server certificate of each device, for `RdpSettings.serverCertificate`: one
/// query per batch of device ids (API-005). A device without one is absent from the map.
pub struct ServerCertificatesByDevice(pub(super) PgPool);

impl Loader<DeviceId> for ServerCertificatesByDevice {
    type Value = ServerCertificate;
    type Error = Arc<Error>;

    async fn load(
        &self,
        keys: &[DeviceId],
    ) -> Result<HashMap<DeviceId, ServerCertificate>, Self::Error> {
        server_certificates(&self.0, keys).await.map_err(Arc::new)
    }
}
