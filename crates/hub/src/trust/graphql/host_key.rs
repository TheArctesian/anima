//! `HostKey` as the GraphQL type K1 describes, and the loader behind `SshSettings.hostKey`
//! (SEC-006, API-005). The key blob is never exposed.

use std::{collections::HashMap, sync::Arc};

use async_graphql::{Object, dataloader::Loader};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    ids::DeviceId,
    trust::{Error, HostKey, host_keys},
};

#[Object]
impl HostKey {
    async fn algorithm(&self) -> &str {
        self.public_key.algorithm()
    }

    /// OpenSSH's `SHA256:` fingerprint of the key.
    async fn fingerprint_sha256(&self) -> String {
        self.public_key.fingerprint().to_string()
    }

    async fn first_seen_at(&self) -> DateTime<Utc> {
        self.first_seen_at
    }
}

/// The accepted host key of each device, for `SshSettings.hostKey`: one query per batch of
/// device ids (API-005). A device without one is absent from the map.
pub struct HostKeysByDevice(pub(super) PgPool);

impl Loader<DeviceId> for HostKeysByDevice {
    type Value = HostKey;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, HostKey>, Self::Error> {
        host_keys(&self.0, keys).await.map_err(Arc::new)
    }
}
