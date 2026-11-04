//! Batched reads that join the checks domain into `Device` (API-005).
//!
//! Each loader makes one call to the checks face per batch (per window for uptime). The
//! status and uptime loaders read the schema clock once per batch, so every device in the
//! batch is judged at the same instant (CHK-004).

use std::{collections::HashMap, net::IpAddr, sync::Arc};

use async_graphql::dataloader::Loader;
use sqlx::PgPool;

use crate::{
    checks::{self, Check, Error, UptimeWindow},
    clock::Clock,
    ids::DeviceId,
    status::Status,
};

/// The status of each device, for `Device.status`.
pub struct StatusOf {
    pub(in crate::devices::graphql) db: PgPool,
    pub(in crate::devices::graphql) clock: Arc<dyn Clock>,
}

impl Loader<DeviceId> for StatusOf {
    type Value = Status;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, Status>, Self::Error> {
        let now = self.clock.now();
        checks::statuses(&self.db, keys, now)
            .await
            .map_err(Arc::new)
    }
}

/// The uptime of each device over a window, for `Device.uptime`. A device with no counted
/// beats has no entry.
pub struct UptimeOf {
    pub(in crate::devices::graphql) db: PgPool,
    pub(in crate::devices::graphql) clock: Arc<dyn Clock>,
}

impl Loader<(DeviceId, UptimeWindow)> for UptimeOf {
    type Value = f64;
    type Error = Arc<Error>;

    async fn load(
        &self,
        keys: &[(DeviceId, UptimeWindow)],
    ) -> Result<HashMap<(DeviceId, UptimeWindow), f64>, Self::Error> {
        let now = self.clock.now();
        let mut by_window: HashMap<UptimeWindow, Vec<DeviceId>> = HashMap::new();
        for (device, window) in keys {
            by_window.entry(*window).or_default().push(*device);
        }
        let mut found = HashMap::new();
        for (window, devices) in by_window {
            let ratios = checks::uptimes(&self.db, &devices, window, now)
                .await
                .map_err(Arc::new)?;
            found.extend(
                ratios
                    .into_iter()
                    .map(|(device, ratio)| ((device, window), ratio)),
            );
        }
        Ok(found)
    }
}

/// The checks of each device in name order, for `Device.checks`.
pub struct ChecksOf(pub(in crate::devices::graphql) PgPool);

impl Loader<DeviceId> for ChecksOf {
    type Value = Vec<Check>;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Check>>, Self::Error> {
        checks::for_devices(&self.0, keys).await.map_err(Arc::new)
    }
}

/// The address the last check resolved for each device, for `Device.ip` when the host is a
/// name.
pub struct ResolvedIpOf(pub(in crate::devices::graphql) PgPool);

impl Loader<DeviceId> for ResolvedIpOf {
    type Value = IpAddr;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, IpAddr>, Self::Error> {
        checks::resolved_ips(&self.0, keys).await.map_err(Arc::new)
    }
}
