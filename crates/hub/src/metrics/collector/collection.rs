//! One device's collection: a link opened for it, its fixed commands run, the reading stored
//! and published, the link dropped (MET-001, MET-002, P2-003, D-11). I/O edge; nothing here
//! logs, and no failure holds a host, an address, command output or a secret (LOG-007).

use std::sync::Arc;

use async_graphql::ID;
use chrono::{DateTime, Utc};
use tokio::time::timeout;

use super::{
    Context, DEADLINE,
    outputs::{self, Parts, Previous},
};
use crate::{
    devices::Device,
    ids::DeviceId,
    metrics::{
        self, MetricSample, MetricsAdded, Reading, Readings, Sample,
        commands::{Command, commands},
        record_collection,
    },
    ssh::{self, LinkError},
};

/// Why a device's collection ended early this round. The texts are fixed (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(super) enum Failure {
    /// The link did not open or a command did not run; the link's text names the step.
    #[error(transparent)]
    Link(LinkError),
    #[error("the device's collection did not finish within 20 seconds")]
    Deadline,
    #[error("the collected metrics cannot be stored or read back")]
    Store(#[source] metrics::Error),
    /// The device was deleted during the round.
    #[error("the device was deleted")]
    Gone,
}

impl From<metrics::Error> for Failure {
    fn from(error: metrics::Error) -> Self {
        match error {
            metrics::Error::NoDevice => Self::Gone,
            other => Self::Store(other),
        }
    }
}

/// How one device's collection ended, for the round to keep.
#[derive(Debug)]
pub(super) struct Finished {
    pub(super) device: DeviceId,
    /// The device's name, for the event that reports a change (D-10).
    pub(super) name: String,
    /// The reading to rate the next collection against, or why there is none this round.
    pub(super) result: Result<Previous, Failure>,
}

/// Collects `device` at the hub clock's time, against its `previous` reading.
pub(super) async fn collect(
    ctx: Arc<Context>,
    device: Device,
    previous: Option<Previous>,
) -> Finished {
    let at = ctx.clock.now();
    let result = collect_at(&ctx, &device, previous.as_ref(), at).await;
    Finished {
        device: device.id,
        name: device.name,
        result,
    }
}

async fn collect_at(
    ctx: &Context,
    device: &Device,
    previous: Option<&Previous>,
    at: DateTime<Utc>,
) -> Result<Previous, Failure> {
    let reading = within_deadline(read(ctx, device, at)).await?;
    let samples = outputs::collected(previous, &reading, at);
    record_collection(&ctx.db, device.id, at, &samples).await?;
    if !samples.is_empty() {
        publish(ctx, device.id, at, &samples).await?;
    }
    Ok(Previous { at, reading })
}

/// The result of `work`, a device's link and commands, or [`Failure::Deadline`] when it takes
/// longer than [`DEADLINE`]; dropping it then drops its link (D-11).
pub(super) async fn within_deadline<T>(
    work: impl Future<Output = Result<T, LinkError>>,
) -> Result<T, Failure> {
    let done = timeout(DEADLINE, work).await;
    done.map_err(|_| Failure::Deadline)?.map_err(Failure::Link)
}

/// Opens a link to `device`, runs its commands in order, each within its limit, reads each
/// output into the reading and drops its bytes before the next command (D-11), then drops the
/// link. The first link error ends the collection.
async fn read(ctx: &Context, device: &Device, at: DateTime<Utc>) -> Result<Reading, LinkError> {
    let Context { db, collecting, .. } = ctx;
    let mut link = ssh::link(db, &collecting.vault, &collecting.policy, device, at).await?;
    let mut parts = Parts::default();
    for Command {
        source,
        fixed,
        limit,
    } in commands(device.os)
    {
        let output = link.run(&fixed, limit).await?;
        parts.add(source, &output);
    }
    Ok(parts.reading(at))
}

/// Publishes what the collection added with the device's readings as stored, so the feed and
/// the `readings` face show the same values. A device whose samples hold no card kind gets
/// readings with no values.
async fn publish(
    ctx: &Context,
    device: DeviceId,
    at: DateTime<Utc>,
    samples: &[Sample],
) -> Result<(), Failure> {
    let readings = metrics::readings(&ctx.db, &[device], at)
        .await?
        .remove(&device)
        .unwrap_or(Readings {
            at,
            cpu_percent: None,
            memory_percent: None,
            disk_percent: None,
            vram_percent: None,
            vram_gpu: None,
        });
    ctx.feed.publish(MetricsAdded {
        device_id: ID(device.to_string()),
        at,
        samples: samples.iter().map(MetricSample::from).collect(),
        readings,
    });
    Ok(())
}
