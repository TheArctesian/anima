//! Batched reads that join the web interfaces domain into `Device` (R-011, API-005): one call
//! to the web interfaces face per batch. The domain's types are the GraphQL objects.

use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::{DataLoader, Loader};
use sqlx::PgPool;

use crate::{
    ids::DeviceId,
    web_interfaces::{self, Error, Suggestion, WebInterface},
};

/// The interfaces of each device in creation order, for `Device.webInterfaces` (WEB-001).
pub struct WebInterfacesOf(PgPool);

impl Loader<DeviceId> for WebInterfacesOf {
    type Value = Vec<WebInterface>;
    type Error = Arc<Error>;

    async fn load(
        &self,
        keys: &[DeviceId],
    ) -> Result<HashMap<DeviceId, Vec<WebInterface>>, Self::Error> {
        web_interfaces::for_devices(&self.0, keys)
            .await
            .map_err(Arc::new)
    }
}

/// The HTTP checks offered as interfaces for each device, for `Device.webInterfaceSuggestions`
/// (WEB-002).
pub struct SuggestionsOf(PgPool);

impl Loader<DeviceId> for SuggestionsOf {
    type Value = Vec<Suggestion>;
    type Error = Arc<Error>;

    async fn load(
        &self,
        keys: &[DeviceId],
    ) -> Result<HashMap<DeviceId, Vec<Suggestion>>, Self::Error> {
        web_interfaces::suggestions(&self.0, keys)
            .await
            .map_err(Arc::new)
    }
}

/// The loader behind `Device.webInterfaces`: one query per batch of device ids. It has no
/// cache, so one instance serves every request.
#[must_use]
pub fn web_interfaces_loader(pool: PgPool) -> DataLoader<WebInterfacesOf> {
    DataLoader::new(WebInterfacesOf(pool), tokio::spawn)
}

/// The loader behind `Device.webInterfaceSuggestions`. It has no cache, so one instance
/// serves every request.
#[must_use]
pub fn web_interface_suggestions_loader(pool: PgPool) -> DataLoader<SuggestionsOf> {
    DataLoader::new(SuggestionsOf(pool), tokio::spawn)
}
