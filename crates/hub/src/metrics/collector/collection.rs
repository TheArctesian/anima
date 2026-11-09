//! One device's collection: a link opened for it, its fixed commands run, then its GPU reads,
//! a new discovery and the samples stored and published, the link dropped (MET-001, MET-002,
//! MET-006, P2-003, D-11). I/O edge; nothing here logs, and no failure holds a host, an
//! address, command output or a secret (LOG-007).

use std::sync::Arc;

use async_graphql::ID;
use chrono::{DateTime, Utc};
use tokio::time::{Instant, timeout};

use super::{
    Context, DEADLINE,
    gpus::{self, Discovered, GpuRound, Plan},
    outputs::{self, Parts, Previous},
};
use crate::{
    devices::Device,
    ids::DeviceId,
    metrics::{
        self, Gpu, MetricSample, MetricsAdded, Reading, Readings, Sample,
        commands::{Command, commands},
        record_collection,
        store::{self, Discovery},
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

/// What the collector keeps of a device's last collection that worked: its reading, for the
/// next rates, and the GPUs known from its last discovery.
#[derive(Clone, Debug)]
pub(super) struct Kept {
    pub(super) previous: Previous,
    pub(super) gpus: Option<Discovered>,
}

/// How one device's collection ended, for the round to keep.
#[derive(Debug)]
pub(super) struct Finished {
    pub(super) device: DeviceId,
    /// The device's name, for the event that reports a change (D-10).
    pub(super) name: String,
    /// What to keep for the next collection, or why there is nothing new this round.
    pub(super) result: Result<Kept, Failure>,
}

/// Collects `device` at the hub clock's time, against what was `kept` of its last collection.
pub(super) async fn collect(ctx: Arc<Context>, device: Device, kept: Option<Kept>) -> Finished {
    let at = ctx.clock.now();
    let result = collect_at(&ctx, &device, kept, at).await;
    Finished {
        device: device.id,
        name: device.name,
        result,
    }
}

async fn collect_at(
    ctx: &Context,
    device: &Device,
    kept: Option<Kept>,
    at: DateTime<Utc>,
) -> Result<Kept, Failure> {
    let (previous, known) = kept.map_or((None, None), |kept| (Some(kept.previous), kept.gpus));
    let (reading, gpus) = within_deadline(read(ctx, device, known, at)).await?;
    if gpus.discovered_now
        && let Some(found) = &gpus.known
    {
        store_discovery(ctx, device.id, found).await?;
    }
    let mut samples = outputs::collected(previous.as_ref(), &reading, at);
    samples.extend(gpus.samples);
    record_collection(&ctx.db, device.id, at, &samples).await?;
    if !samples.is_empty() {
        publish(ctx, device.id, at, &samples).await?;
    }
    Ok(Kept {
        previous: Previous { at, reading },
        gpus: gpus.known,
    })
}

/// Replaces the stored GPUs of `device` with those `found` (MET-006).
async fn store_discovery(
    ctx: &Context,
    device: DeviceId,
    found: &Discovered,
) -> Result<(), Failure> {
    let gpus: Vec<Gpu> = found.gpus.iter().map(|found| found.gpu.clone()).collect();
    let discovery = Discovery {
        found_at: found.found_at,
        boot_at: found.boot_at,
        gpus: &gpus,
    };
    store::replace_gpus(&ctx.db, device, discovery).await?;
    Ok(())
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
/// output into the reading and drops its bytes before the next command (D-11), then runs the
/// GPU reads its plan asks for until the context's GPU window after the start, then drops the
/// link. The first link error of the commands ends the collection; a GPU read never does.
async fn read(
    ctx: &Context,
    device: &Device,
    known: Option<Discovered>,
    at: DateTime<Utc>,
) -> Result<(Reading, GpuRound), LinkError> {
    let gpu_until = Instant::now() + ctx.gpu_window;
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
    let reading = parts.reading(at);
    let plan = Plan::new(device.os, known, &reading, at);
    let answers = gpus::ask(&mut link, plan.commands(), gpu_until).await;
    Ok((reading, plan.finish(&answers)))
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
        samples: samples.iter().filter_map(MetricSample::served).collect(),
        readings,
    });
    Ok(())
}
