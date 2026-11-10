//! Batched reads that join the metrics domain into `Device` (API-005, MET-004, MET-006,
//! MET-018): one call to the metrics face per batch. `readings` is judged at the schema clock, read once
//! per batch.

use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::{DataLoader, Loader};
use sqlx::PgPool;

use crate::{
    clock::Clock,
    ids::DeviceId,
    metrics::{self, Error, Gpu, Readings},
};

/// The current readings of each device, for `Device.readings` (MET-004). A device without a
/// recent sample has no entry.
pub struct ReadingsOf {
    db: PgPool,
    clock: Arc<dyn Clock>,
}

impl Loader<DeviceId> for ReadingsOf {
    type Value = Readings;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, Readings>, Self::Error> {
        let now = self.clock.now();
        metrics::readings(&self.db, keys, now)
            .await
            .map_err(Arc::new)
    }
}

/// The GPUs of each device by index, for `Device.gpus` (MET-006).
pub struct GpusOf(PgPool);

impl Loader<DeviceId> for GpusOf {
    type Value = Vec<Gpu>;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Gpu>>, Self::Error> {
        metrics::gpus(&self.0, keys).await.map_err(Arc::new)
    }
}

/// Whether each agent device's metrics are stale, for `Device.metricsStale` (MET-018). Load
/// only devices whose metrics source is `agent`.
pub struct StaleOf(PgPool);

impl Loader<DeviceId> for StaleOf {
    type Value = bool;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[DeviceId]) -> Result<HashMap<DeviceId, bool>, Self::Error> {
        let stale = metrics::stale_agents(&self.0, keys)
            .await
            .map_err(Arc::new)?;
        Ok(keys.iter().map(|id| (*id, stale.contains(id))).collect())
    }
}

/// The loader behind `Device.readings`: one `metrics::readings` call per batch, no cache.
#[must_use]
pub fn readings_loader(pool: PgPool, clock: Arc<dyn Clock>) -> DataLoader<ReadingsOf> {
    DataLoader::new(ReadingsOf { db: pool, clock }, tokio::spawn)
}

/// The loader behind `Device.gpus`: one `metrics::gpus` call per batch, no cache.
#[must_use]
pub fn gpus_loader(pool: PgPool) -> DataLoader<GpusOf> {
    DataLoader::new(GpusOf(pool), tokio::spawn)
}

/// The loader behind `Device.metricsStale`: one `metrics::stale_agents` call per batch, no cache.
#[must_use]
pub fn stale_loader(pool: PgPool) -> DataLoader<StaleOf> {
    DataLoader::new(StaleOf(pool), tokio::spawn)
}
