//! The GPUs of one collection (MET-006 to MET-009, D-30): whether it looks for the device's
//! GPUs, which vendors' fixed reads it runs, and the samples of the GPUs it knows.
//!
//! `discovery` decides when to look and numbers what it finds, `answers` reads each vendor's
//! output, `sysfs` splits the AMD files by card and `samples` makes the samples; all pure.
//! `stage` runs the reads on the link.

pub(super) mod answers;
pub(super) mod discovery;
pub(super) mod samples;
mod stage;
pub(super) mod sysfs;

use chrono::{DateTime, Utc};

pub(super) use answers::Answer;
pub(super) use discovery::Discovered;
pub(super) use stage::ask;

use crate::{
    devices::OperatingSystem,
    metrics::{
        GpuVendor, Reading, Sample,
        commands::{GpuCommand, gpu_commands},
    },
};

/// What one collection does about GPUs, decided from its reading before the GPU reads run.
pub(super) struct Plan {
    at: DateTime<Utc>,
    boot_at: Option<DateTime<Utc>>,
    looking: bool,
    known: Option<Discovered>,
    commands: Vec<GpuCommand>,
}

/// The GPU part of a collection: what the hub now knows of the device's GPUs, whether this
/// collection found them anew (to be stored), and the samples.
#[derive(Debug)]
pub(super) struct GpuRound {
    pub(super) known: Option<Discovered>,
    pub(super) discovered_now: bool,
    pub(super) samples: Vec<Sample>,
}

impl Plan {
    /// The plan of the collection taken at `at` of a device running `os`, whose `reading`
    /// gives its boot time, with the GPUs `known` from its last discovery. A collection that
    /// looks runs every vendor's read for `os`; another runs only the reads of the vendors
    /// known, so a device without GPUs runs none.
    pub(super) fn new(
        os: OperatingSystem,
        known: Option<Discovered>,
        reading: &Reading,
        at: DateTime<Utc>,
    ) -> Self {
        let boot_at = discovery::boot_at(reading, at);
        let looking = discovery::due(known.as_ref(), boot_at, at);
        let mut commands = gpu_commands(os);
        if !looking {
            let vendors: Vec<GpuVendor> = known
                .iter()
                .flat_map(|known| &known.gpus)
                .map(|found| found.gpu.vendor)
                .collect();
            commands.retain(|command| vendors.contains(&command.vendor));
        }
        Self {
            at,
            boot_at,
            looking,
            known,
            commands,
        }
    }

    /// The reads to run, in order.
    pub(super) fn commands(&self) -> &[GpuCommand] {
        &self.commands
    }

    /// The round's GPU part from the `answers` of [`Plan::commands`]. A look with an unknown
    /// answer keeps the GPUs known before, and the next collection looks again.
    pub(super) fn finish(self, answers: &[(GpuVendor, Answer)]) -> GpuRound {
        let found = self
            .looking
            .then(|| discovery::discovered(answers, self.at, self.boot_at))
            .flatten();
        let discovered_now = found.is_some();
        let known = found.or(self.known);
        let samples = known
            .as_ref()
            .map(|known| samples::samples(known, answers))
            .unwrap_or_default();
        GpuRound {
            known,
            discovered_now,
            samples,
        }
    }
}
