//! The GPUs of one collection (MET-006 to MET-009, D-30): whether it looks for the device's
//! GPUs, which vendors' fixed reads it runs, and the samples of the GPUs it knows.
//!
//! `discovery` decides when to look and numbers what it finds, `answers` reads each vendor's
//! output, `sysfs` splits the AMD files by card and `samples` makes the samples; all pure.
//! `reads` runs the reads on the link.

pub(super) mod answers;
pub(super) mod discovery;
mod reads;
pub(super) mod samples;
pub(super) mod sysfs;

use chrono::{DateTime, Utc};

pub(super) use answers::Answer;
pub(super) use discovery::Discovered;
pub(super) use reads::ask;

use crate::{
    devices::OperatingSystem,
    metrics::{
        GpuVendor, Reading, Sample,
        commands::{GpuCommand, gpu_commands},
    },
};
use discovery::Look;

/// What one collection does about GPUs, decided from its reading before the GPU reads run.
pub(super) struct Plan {
    look: Option<Look>,
    known: Option<Discovered>,
    commands: Vec<GpuCommand>,
}

/// The GPU part of a collection: what the hub now knows of the device's GPUs, whether this
/// collection looked (so what it knows is to be stored), and the samples.
#[derive(Debug)]
pub(super) struct GpuRound {
    pub(super) known: Option<Discovered>,
    pub(super) discovered_now: bool,
    pub(super) samples: Vec<Sample>,
}

impl Plan {
    /// The plan of the collection taken at `at` of a device running `os`, whose `reading`
    /// gives its boot time, with the GPUs `known` before. It runs the reads of the vendors
    /// known and of those its look includes, so a device without GPUs that is not looked at
    /// runs none.
    pub(super) fn new(
        os: OperatingSystem,
        known: Option<Discovered>,
        reading: &Reading,
        at: DateTime<Utc>,
    ) -> Self {
        let look = discovery::due(known.as_ref(), discovery::boot_at(reading, at), at);
        let vendors: Vec<GpuVendor> = known
            .iter()
            .flat_map(|known| &known.gpus)
            .map(|found| found.gpu.vendor)
            .collect();
        let mut commands = gpu_commands(os);
        commands.retain(|command| {
            vendors.contains(&command.vendor)
                || look
                    .as_ref()
                    .is_some_and(|look| look.includes(command.vendor))
        });
        Self {
            look,
            known,
            commands,
        }
    }

    /// The reads to run, in order.
    pub(super) fn commands(&self) -> &[GpuCommand] {
        &self.commands
    }

    /// The round's GPU part from the `answers` of [`Plan::commands`]: after a look, what it
    /// found merged into what was known; otherwise what was known.
    pub(super) fn finish(self, answers: &[(GpuVendor, Answer)]) -> GpuRound {
        let discovered_now = self.look.is_some();
        let known = match &self.look {
            Some(look) => Some(look.merge(self.known.as_ref(), answers)),
            None => self.known,
        };
        let samples = known
            .as_ref()
            .map_or_default(|known| samples::samples(known, answers));
        GpuRound {
            known,
            discovered_now,
            samples,
        }
    }
}
