//! One accepted batch written in one transaction: the agent's mark, the GPU set, the series,
//! the samples and their aggregates, and the event of an agent that is fresh again (K8 Section
//! 4; MET-006, MET-015, MET-018; R-098 a, b, e).
//!
//! The device row is taken first, the order a device delete and a revoke take the rows in, so
//! none of them deadlock, and two batches of one device are written one after the other.

use agent_core::wire::{RefusalCode, TokenHash};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::samples::Rows;
use crate::{
    devices::{self, MetricsSource},
    events::{self, Bus, NewEvent, Severity, Source},
    ids::DeviceId,
    metrics::{
        Gpu,
        ingest::{Error, store},
        store::{self as metrics_store, Discovery},
    },
};

/// How far back a series counts as live: the oldest point a batch may hold (MET-017).
const LIVE_SPAN: TimeDelta = TimeDelta::hours(1);

/// The most series of one device with a sample in the last hour, a batch's own included
/// (R-098 e). One agent sends at most 519 at a time (one per plain kind, `MAX_LABELS` per disk
/// and network kind, one per GPU kind for each of the 64 GPU indexes K8 allows); the rest is
/// room for disks and interfaces that come and go within the hour.
pub(super) const MAX_LIVE_SERIES: i64 = 1024;

/// The batch's device and credential, and its GPUs when it listed them.
pub(super) struct Push<'a> {
    pub(super) device: DeviceId,
    /// The device's name, for the event of an agent that is fresh again.
    pub(super) name: &'a str,
    /// The token the batch came with; it must still be the device's usable token.
    pub(super) token: TokenHash,
    /// `None`: the GPU set is unchanged; `Some(&[])`: the device has no GPUs now (R-098 b).
    pub(super) gpus: Option<&'a [Gpu]>,
}

/// What became of a batch that reached the store.
pub(super) enum Written {
    /// Stored, with how many of its samples were new.
    Kept(i64),
    /// Not stored: the source or the token changed since the batch was checked
    /// (`TOKEN_REFUSED`), or it would pass [`MAX_LIVE_SERIES`] (`MALFORMED`).
    Refused(RefusalCode),
}

/// Writes the batch accepted at `now`, or nothing when it is refused here.
///
/// # Errors
/// `Database`, `Devices`, `Metrics` or `Events` when a read or write fails; nothing is stored.
pub(super) async fn write(
    db: &PgPool,
    bus: &Bus,
    (push, rows): (Push<'_>, &Rows),
    now: DateTime<Utc>,
) -> Result<Written, Error> {
    let mut tx = db.begin().await?;
    match devices::lock_metrics_source(&mut tx, push.device).await {
        Ok(MetricsSource::Agent) => {}
        Ok(MetricsSource::None | MetricsSource::Ssh) | Err(devices::Error::NotFound(_)) => {
            return Ok(Written::Refused(RefusalCode::TokenRefused));
        }
        Err(other) => return Err(Error::Devices(other)),
    }
    let Some(was_stale) = store::take_batch(&mut tx, push.device, push.token, now).await? else {
        return Ok(Written::Refused(RefusalCode::TokenRefused));
    };
    let live =
        metrics_store::live_series(&mut tx, push.device, now - LIVE_SPAN, &rows.series).await?;
    if live > MAX_LIVE_SERIES {
        return Ok(Written::Refused(RefusalCode::Malformed));
    }
    if let Some(gpus) = push.gpus {
        // The agent sends no boot time: its own discovery cadence (MET-006) replaces the
        // collector's boot check, which never runs for an agent device.
        let discovery = Discovery {
            found_at: now,
            boot_at: None,
            gpus,
        };
        metrics_store::write_gpus(&mut tx, push.device, discovery).await?;
    }
    metrics_store::add_series(&mut tx, push.device, &rows.series).await?;
    let kept = metrics_store::add_points(&mut tx, push.device, &rows.values).await?;
    if was_stale {
        events::record(&mut tx, bus, now, fresh_again(push.device, push.name)).await?;
    }
    tx.commit().await?;
    Ok(Written::Kept(kept))
}

/// The one event of an agent that sends again after it was marked stale (MET-018).
fn fresh_again(device: DeviceId, name: &str) -> NewEvent {
    NewEvent {
        severity: Severity::Info,
        source: Source::Metrics,
        device: Some((device, name.to_owned())),
        message: format!("The agent on {name} sends metrics again."),
    }
}
