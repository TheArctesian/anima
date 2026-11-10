//! Resource metrics of a device: CPU, memory, disk, network, uptime and load, read from the
//! output of fixed read-only commands run over SSH (spec 6.6, HUB-004, MET-001, MET-002), and
//! the device's GPUs with their usage (MET-006).
//!
//! The pure part: [`agent_core::readings`] turns command output into a [`Reading`], and
//! [`samples`] turns two readings and the time between them into [`Sample`]s. The device is
//! not trusted, so no reader panics and none fails on a bad line. [`spawn_collector`] runs the
//! commands every 30 seconds and publishes each collection on the [`Feed`]. A device may push
//! its metrics from the Anima agent instead ([`agent_router`], K8); [`spawn_stale_watch`] marks
//! an agent that stopped pushing (MET-018).

mod collector;
mod commands;
mod feed;
mod gpu_label;
pub mod graphql;
mod ingest;
mod latest;
mod model;
mod range;
mod retention;
mod store;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Days, TimeDelta, Utc};
use sqlx::PgPool;

pub use agent_core::{
    points::{MAX_LABELS, Sample, samples},
    readings::{Cpu, Disk, Interface, Load, Memory, Reading},
};
pub use collector::{Collecting, spawn_collector};
pub use feed::Feed;
pub use ingest::{agent_router, spawn_stale_watch};
pub use model::{
    Gpu, GpuVendor, MetricKind, MetricPoint, MetricRange, MetricSample, MetricSeries, MetricsAdded,
    Readings,
};
pub use retention::{Retention, Trimmed};

use crate::ids::DeviceId;

/// How long a sample counts as a device's current reading: four collections (D-8).
const READING_WINDOW: TimeDelta = TimeDelta::minutes(2);

/// How many days of sample partitions exist ahead of today (DB-004, D-6).
const DAYS_AHEAD: i32 = 7;

/// Why a metrics operation failed. Messages hold no SQL and no input value.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// The device was deleted before its samples were written.
    #[error("the device does not exist")]
    NoDevice,
    /// A stored row holds a kind or vendor this version does not know.
    #[error("a stored metric row cannot be read")]
    Unreadable,
}

/// Writes the `samples` of one collection of `device` taken at `at`: the series they belong to
/// (created on first use), the samples, and the minute, hour and day aggregates, together
/// (DB-008). A sample already stored for its series and time is left as it is.
///
/// # Errors
/// `NoDevice` when the device does not exist; `Database` when a query fails or `at` is outside
/// the window the database makes partitions for.
pub async fn record_collection(
    db: &PgPool,
    device: DeviceId,
    at: DateTime<Utc>,
    samples: &[Sample],
) -> Result<(), Error> {
    if samples.is_empty() {
        return Ok(());
    }
    store::record(db, device, at, samples).await
}

/// The current readings of each of `devices` that has a sample taken in the last 2 minutes
/// before `now` (MET-004, D-8). A device without one has no entry.
///
/// # Errors
/// `Database` when the query fails; `Unreadable` for a stored kind this version does not know.
pub async fn readings(
    db: &PgPool,
    devices: &[DeviceId],
    now: DateTime<Utc>,
) -> Result<HashMap<DeviceId, Readings>, Error> {
    let latest = store::latest(db, devices, now, READING_WINDOW).await?;
    Ok(latest::readings(latest))
}

/// The series of `device` over `range` ending at `now`, with their points oldest first (MET-003,
/// D-7). A series without a point in the range is left out.
///
/// # Errors
/// `Database` when the query fails; `Unreadable` for a stored kind this version does not know.
pub async fn series(
    db: &PgPool,
    device: DeviceId,
    range: MetricRange,
    now: DateTime<Utc>,
) -> Result<Vec<MetricSeries>, Error> {
    store::chart(db, device, range::window(range, now)).await
}

/// The GPUs stored for each of `devices`, by index; a device with none has no entry (MET-006).
///
/// # Errors
/// `Database` when the query fails; `Unreadable` for a stored vendor this version does not know.
pub async fn gpus(db: &PgPool, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Gpu>>, Error> {
    store::gpus(db, devices).await
}

/// Of `devices`, those whose agent's metrics are stale: its token was revoked, it has sent no
/// batch yet, or it was marked stale after 30 seconds with none (MET-018). Ask only for the
/// devices whose metrics source is `agent`; for any other the answer means nothing.
///
/// # Errors
/// `Database` when the query fails.
pub async fn stale_agents(db: &PgPool, devices: &[DeviceId]) -> Result<HashSet<DeviceId>, Error> {
    Ok(ingest::without_fresh_agent(db, devices).await?)
}

/// Creates the sample partitions of today (UTC) and the next 7 days (DB-004).
///
/// # Errors
/// `Database` when a query fails, for instance when `now` is later than the database clock by
/// more than the partition window allows.
pub async fn create_partitions_ahead(db: &PgPool, now: DateTime<Utc>) -> Result<(), Error> {
    store::create_days_ahead(db, retention::start_of_day(now), DAYS_AHEAD).await
}

/// The daily retention run: drops sample partitions past `retention.samples` (DB-005), deletes
/// aggregate rows past their periods (DB-009), the samples of deleted series (DEV-015) and the
/// series that went idle (D-5).
/// `now` is held to the database clock, so a caller cannot ask for more to go than the
/// database's own time allows (CQ-021).
///
/// # Errors
/// `Database` when a query fails, or when the database refuses to drop a partition still
/// inside the 1-day floor.
pub async fn drop_expired_partitions(
    db: &PgPool,
    now: DateTime<Utc>,
    retention: Retention,
) -> Result<Trimmed, Error> {
    let now = now.min(store::database_now(db).await?);
    let cutoffs = retention.cutoffs(now);
    let partitions = store::drop_days_before(db, cutoffs.samples, now).await?;
    let [minute, hour, day] = store::trim_aggregates(db, &cutoffs).await?;
    let idle_series = store::delete_idle_series(db, cutoffs.minute).await?;
    let orphan_samples = store::delete_orphan_samples(db).await?;
    Ok(Trimmed {
        partitions,
        minute,
        hour,
        day,
        orphan_samples,
        idle_series,
    })
}

impl Error {
    /// For the process log: what failed, as a fixed word, never the source text (LOG-007).
    #[must_use]
    pub const fn fault(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::NoDevice => "no_device",
            Self::Unreadable => "unreadable",
        }
    }
}

/// The daily job: creates the partitions ahead, then drops what retention allows (DB-004,
/// DB-005, D-5, D-25). Samples are kept `samples_keep` (the operator's period); the aggregates
/// stay at their fixed periods (DB-009). Run by `upkeep.rs` with the other retention jobs.
///
/// # Errors
/// What [`create_partitions_ahead`] or [`drop_expired_partitions`] returns.
pub async fn keep_partitions(
    db: &PgPool,
    now: DateTime<Utc>,
    samples_keep: Days,
) -> Result<(), Error> {
    create_partitions_ahead(db, now).await?;
    let samples = now - now.checked_sub_days(samples_keep).unwrap_or(now);
    let retention = Retention {
        samples,
        ..Retention::default()
    };
    drop_expired_partitions(db, now, retention).await?;
    Ok(())
}
