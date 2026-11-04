//! Batched reads behind `Check.status`, `Check.lastHeartbeat` and `Check.uptime`: a fixed number
//! of queries per level of a nested selection, however many checks it holds (API-005).
//!
//! None of the loaders caches, so one instance serves every request. The ones that judge
//! time read the clock once per batch.

use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::{DataLoader, Loader};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    checks::{Error, Heartbeat, UptimeWindow, from_devices, status, store, uptime},
    clock::Clock,
    devices,
    ids::{CheckId, DeviceId},
    maintenance,
    status::Status,
};

/// The status each check shows, with pause, groups, maintenance and the device's manual flag
/// applied: the rules of `device_status`, for many checks at once (CHK-004, CHK-006).
pub struct CheckStatuses {
    db: PgPool,
    clock: Arc<dyn Clock>,
}

impl Loader<CheckId> for CheckStatuses {
    type Value = Status;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[CheckId]) -> Result<HashMap<CheckId, Status>, Self::Error> {
        let now = self.clock.now();
        let owners = store::devices_of(&self.db, keys).await.map_err(Arc::new)?;
        let mut requested: Vec<DeviceId> = Vec::new();
        for (_, device) in &owners {
            if !requested.contains(device) {
                requested.push(*device);
            }
        }
        let standings = store::standings(&self.db, &requested)
            .await
            .map_err(Arc::new)?;
        let known = devices::list(&self.db)
            .await
            .map_err(|e| Arc::new(from_devices(e)))?;
        let active = maintenance::active_targets(&self.db, now)
            .await
            .map_err(|e| Arc::new(Error::Maintenance(e)))?;
        let mut by_device: HashMap<DeviceId, Vec<status::Standing>> = HashMap::new();
        for standing in standings {
            by_device.entry(standing.device).or_default().push(standing);
        }
        Ok(owners
            .into_iter()
            .filter_map(|(check, owner)| {
                let device = known.iter().find(|device| device.id == owner)?;
                let own = by_device.get(&owner).map_or(&[][..], Vec::as_slice);
                Some((check, status::of_check(check, device, own, &active)))
            })
            .collect())
    }
}

/// The newest heartbeat of each check that has one.
pub struct LastHeartbeats(PgPool);

impl Loader<CheckId> for LastHeartbeats {
    type Value = Heartbeat;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[CheckId]) -> Result<HashMap<CheckId, Heartbeat>, Self::Error> {
        let latest = store::latest_heartbeats(&self.0, keys)
            .await
            .map_err(Arc::new)?;
        Ok(latest.into_iter().collect())
    }
}

/// A page of heartbeats: how many, and the instant they must be before.
pub type PageKey = (CheckId, u32, Option<DateTime<Utc>>);

/// Pages of heartbeats, newest first. Keys with the same size and cursor are loaded together,
/// so a page for many checks costs one statement.
pub struct HeartbeatPages(PgPool);

impl Loader<PageKey> for HeartbeatPages {
    type Value = Vec<Heartbeat>;
    type Error = Arc<Error>;

    async fn load(
        &self,
        keys: &[PageKey],
    ) -> Result<HashMap<PageKey, Vec<Heartbeat>>, Self::Error> {
        let mut shapes: Vec<(u32, Option<DateTime<Utc>>)> = Vec::new();
        for (_, limit, before) in keys {
            if !shapes.contains(&(*limit, *before)) {
                shapes.push((*limit, *before));
            }
        }
        let mut pages: HashMap<PageKey, Vec<Heartbeat>> = HashMap::new();
        for (limit, before) in shapes {
            let checks: Vec<CheckId> = keys
                .iter()
                .filter(|key| key.1 == limit && key.2 == before)
                .map(|key| key.0)
                .collect();
            let rows = store::heartbeat_pages(&self.0, &checks, before, i64::from(limit))
                .await
                .map_err(Arc::new)?;
            for (check, heartbeat) in rows {
                pages
                    .entry((check, limit, before))
                    .or_default()
                    .push(heartbeat);
            }
        }
        Ok(pages)
    }
}

/// The uptime of each check over a window, from the aggregate tables; a check with no counted
/// beats is absent (CHK-011).
pub struct CheckUptimes {
    db: PgPool,
    clock: Arc<dyn Clock>,
}

impl Loader<(CheckId, UptimeWindow)> for CheckUptimes {
    type Value = f64;
    type Error = Arc<Error>;

    async fn load(
        &self,
        keys: &[(CheckId, UptimeWindow)],
    ) -> Result<HashMap<(CheckId, UptimeWindow), f64>, Self::Error> {
        let now = self.clock.now();
        let mut windows: Vec<UptimeWindow> = Vec::new();
        for (_, window) in keys {
            if !windows.contains(window) {
                windows.push(*window);
            }
        }
        let mut ratios = HashMap::new();
        for window in windows {
            let checks: Vec<CheckId> = keys
                .iter()
                .filter(|(_, wanted)| *wanted == window)
                .map(|(check, _)| *check)
                .collect();
            let span = (window.start(now), now);
            let tallies = store::check_tallies(&self.db, &checks, window.resolution(), span)
                .await
                .map_err(Arc::new)?;
            for (check, tally) in tallies {
                if let Some(ratio) = uptime::ratio(tally) {
                    ratios.insert((check, window), ratio);
                }
            }
        }
        Ok(ratios)
    }
}

/// The loader behind `Check.status`: a few queries per batch of check ids, however many.
#[must_use]
pub fn check_status_loader(pool: PgPool, clock: Arc<dyn Clock>) -> DataLoader<CheckStatuses> {
    DataLoader::new(CheckStatuses { db: pool, clock }, tokio::spawn)
}

/// The loader behind `Check.lastHeartbeat`: one query per batch of check ids.
#[must_use]
pub fn last_heartbeat_loader(pool: PgPool) -> DataLoader<LastHeartbeats> {
    DataLoader::new(LastHeartbeats(pool), tokio::spawn)
}

/// The loader behind `Check.heartbeats`: one statement per page size and cursor per batch.
#[must_use]
pub fn heartbeats_loader(pool: PgPool) -> DataLoader<HeartbeatPages> {
    DataLoader::new(HeartbeatPages(pool), tokio::spawn)
}

/// The loader behind `Check.uptime`: one query per window per batch of check ids.
#[must_use]
pub fn check_uptime_loader(pool: PgPool, clock: Arc<dyn Clock>) -> DataLoader<CheckUptimes> {
    DataLoader::new(CheckUptimes { db: pool, clock }, tokio::spawn)
}
