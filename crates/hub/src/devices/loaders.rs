//! Batched reads for GraphQL: one query per level of a nested selection (API-005).

use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::Loader;
use sqlx::PgPool;

use super::{
    Error,
    model::{Device, Site},
    store,
};
use crate::ids::SiteId;

/// Sites by id, for `Device.site`.
pub struct SitesById(pub(super) PgPool);

impl Loader<SiteId> for SitesById {
    type Value = Site;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[SiteId]) -> Result<HashMap<SiteId, Site>, Self::Error> {
        let sites = store::sites_by_ids(&self.0, keys).await.map_err(Arc::new)?;
        Ok(sites.into_iter().map(|site| (site.id, site)).collect())
    }
}

/// The devices of each site in name byte order, for `Site.devices`.
pub struct DevicesBySite(pub(super) PgPool);

impl Loader<SiteId> for DevicesBySite {
    type Value = Vec<Device>;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[SiteId]) -> Result<HashMap<SiteId, Vec<Device>>, Self::Error> {
        let devices = store::devices_by_sites(&self.0, keys)
            .await
            .map_err(Arc::new)?;
        // The store returns name order; grouping by site keeps that order within each site.
        let mut by_site: HashMap<SiteId, Vec<Device>> = HashMap::new();
        for device in devices {
            by_site.entry(device.site).or_default().push(device);
        }
        Ok(by_site)
    }
}
