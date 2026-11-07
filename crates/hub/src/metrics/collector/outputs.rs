//! What the outputs of one collection give: a [`Reading`] from the readers of each source, and
//! its samples against the device's previous reading (MET-002, D-4). Pure.

use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::{
    metrics::{
        Memory, Reading, Sample,
        commands::Source,
        parse::{
            df, linux,
            macos::{self, Sysctl},
        },
        samples,
    },
    sessions::CommandOutput,
};

/// The reading of a device's earlier collection and the hub time it was taken.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Previous {
    pub(super) at: DateTime<Utc>,
    pub(super) reading: Reading,
}

/// The output's bytes when they can be read: complete and exit status 0. Output cut at the
/// link's cap, a non-zero status or no status means the command did not finish its work, so
/// its source is left out and the others still count.
fn usable(output: &CommandOutput) -> Option<&[u8]> {
    let finished = !output.cut && output.exit_status == Some(0);
    finished.then_some(output.stdout.as_slice())
}

/// A reading built one command output at a time, so the collector keeps only what each
/// output gave and drops its bytes before the next command runs (D-11). Linux and macOS
/// sources fill the same fields; a device sends only the ones its commands ask for.
#[derive(Debug, Default)]
pub(super) struct Parts {
    reading: Reading,
    sysctl: Option<Sysctl>,
    /// macOS memory in use, from `vm_stat`, waiting for the total from `sysctl`.
    used_bytes: Option<u64>,
}

impl Parts {
    /// Reads `output` of `source`'s command into its fields. An unusable output leaves them
    /// empty. `vm_stat` takes the page size from a `sysctl` read before it (the command
    /// table runs `sysctl` first) when its header has none.
    pub(super) fn add(&mut self, source: Source, output: &CommandOutput) {
        let Some(bytes) = usable(output) else {
            return;
        };
        let reading = &mut self.reading;
        match source {
            Source::Stat => reading.cpu = linux::cpu(bytes),
            Source::Top => reading.cpu = macos::cpu(bytes),
            Source::Meminfo => reading.memory = linux::memory(bytes),
            Source::NetDev => reading.interfaces = linux::interfaces(bytes),
            Source::Netstat => reading.interfaces = macos::interfaces(bytes),
            Source::Uptime => reading.uptime_seconds = linux::uptime_seconds(bytes),
            Source::Loadavg => reading.load = linux::load_average(bytes),
            Source::DiskBytes => reading.disks = df::disks(bytes, 1),
            Source::DiskKilobytes => reading.disks = df::disks(bytes, 1024),
            Source::Sysctl => self.sysctl = Some(macos::sysctl(bytes)),
            Source::VmStat => {
                let page_size = self.sysctl.and_then(|sysctl| sysctl.page_size);
                self.used_bytes = macos::memory_used_bytes(bytes, page_size);
            }
        }
    }

    /// The reading of the collection taken at `at`, which dates the macOS boot time.
    pub(super) fn reading(self, at: DateTime<Utc>) -> Reading {
        let Self {
            mut reading,
            sysctl,
            used_bytes,
        } = self;
        let Some(sysctl) = sysctl else {
            return reading;
        };
        reading.load = reading.load.or(sysctl.load);
        reading.uptime_seconds = reading.uptime_seconds.or_else(|| sysctl.uptime_seconds(at));
        if let (None, Some(total_bytes), Some(used_bytes)) =
            (reading.memory, sysctl.memory_bytes, used_bytes)
        {
            reading.memory = Some(Memory {
                total_bytes,
                used_bytes,
            });
        }
        reading
    }
}

/// The samples of `reading` taken at `at`, with rates over the hub time since `previous`. A
/// previous reading at or after `at` (the hub clock went back) gives no rates (D-4).
pub(super) fn collected(
    previous: Option<&Previous>,
    reading: &Reading,
    at: DateTime<Utc>,
) -> Vec<Sample> {
    let elapsed = previous
        .and_then(|previous| (at - previous.at).to_std().ok())
        .unwrap_or(Duration::ZERO);
    samples(previous.map(|previous| &previous.reading), reading, elapsed)
}
