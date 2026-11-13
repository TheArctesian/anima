//! One accepted batch written in one transaction: the agent's mark, the GPU set, the drives, the series,
//! the samples and their aggregates, and the event of an agent that is fresh again (K8 Section
//! 4; MET-006, MET-015, MET-018; R-098 a, b, e).
//!
//! The device row is taken first, the order a device delete and a revoke take the rows in, so
//! none of them deadlock, and two batches of one device are written one after the other.

use agent_core::{
    readings::Drive,
    wire::{RefusalCode, TokenHash},
};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::samples::Rows;
use crate::{
    fleet::{
        devices::{self, MetricsSource},
        events::{self, ActorKind, Bus, NewEvent, Severity, Source},
        metrics::{
            Gpu,
            ingest::{Error, store},
            store::{self as metrics_store, Discovery},
        },
    },
    ids::DeviceId,
};

/// The span in which a device may create at most [`MAX_NEW_SERIES`] series.
const NEW_SERIES_SPAN: TimeDelta = TimeDelta::hours(1);

/// The most series one device may create in any hour, a batch's new ones included (R-098 e,
/// CR-20). Counted by the hub's creation time, so the times of a batch's points cannot reset it.
/// One agent has at most 615 series (at most 256 in one batch): one per plain kind, `MAX_LABELS`
/// per disk, network and drive kind, one per GPU kind for each of the 64 GPU indexes K8 allows;
/// the rest is room for disks, interfaces and drives that come and go within the hour.
pub(super) const MAX_NEW_SERIES: i64 = 1024;

/// The batch's device and credential, and its GPUs and drives when it listed them.
pub(super) struct Push<'a> {
    pub(super) device: DeviceId,
    /// The device's name, for the event of an agent that is fresh again.
    pub(super) name: &'a str,
    /// The token the batch came with; it must still be the device's usable token.
    pub(super) token: TokenHash,
    /// `None`: the GPU set is unchanged; `Some(&[])`: the device has no GPUs now (R-098 b).
    pub(super) gpus: Option<&'a [Gpu]>,
    /// `None`: the drives are unchanged; `Some(&[])`: the device has no drives now (MET-027).
    pub(super) drives: Option<&'a [Drive]>,
}

/// What became of a batch that reached the store.
pub(super) enum Written {
    /// Stored, with how many of its samples were new.
    Kept(i64),
    /// Not stored: the source or the token changed since the batch was checked
    /// (`TOKEN_REFUSED`), or it would pass [`MAX_NEW_SERIES`] (`MALFORMED`).
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
    // Checked again under the device row lock: an operator's switch away from the agent that
    // committed after the first check, while this batch waited for the row, must refuse it.
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
    let since = now - NEW_SERIES_SPAN;
    let created = metrics_store::created_series(&mut tx, push.device, since, &rows.series).await?;
    if created > MAX_NEW_SERIES {
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
    if let Some(drives) = push.drives {
        metrics_store::write_drives(&mut tx, push.device, drives, now).await?;
    }
    metrics_store::add_series(&mut tx, push.device, &rows.series, now).await?;
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
        actor: ActorKind::Hub,
    }
}
