//! What the outputs of one collection give: a [`Reading`] from the readers of each source, and
//! its samples against the device's previous reading (MET-002, D-4). Pure.

use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::{
    metrics::{
        Memory, Reading, Sample,
        commands::Source,
        parse::{df, linux, macos},
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

/// The output of `source`'s command when it can be read: present, complete and exit status 0.
/// Output cut at the link's cap, a non-zero status or no status means the command did not
/// finish its work, so its source is left out and the others still count.
fn usable(outputs: &[(Source, CommandOutput)], source: Source) -> Option<&[u8]> {
    let (_, output) = outputs.iter().find(|(from, _)| *from == source)?;
    let finished = !output.cut && output.exit_status == Some(0);
    finished.then_some(output.stdout.as_slice())
}

/// The reading of one collection taken at `at`. Linux and macOS sources fill the same fields;
/// a device sends only the ones its commands ask for. `at` dates the macOS boot time.
pub(super) fn reading(outputs: &[(Source, CommandOutput)], at: DateTime<Utc>) -> Reading {
    let output = |source| usable(outputs, source);
    let sysctl = output(Source::Sysctl).map(macos::sysctl);
    let mac_memory = sysctl.and_then(|sysctl| {
        let total_bytes = sysctl.memory_bytes?;
        let used = output(Source::VmStat)?;
        let used_bytes = macos::memory_used_bytes(used, sysctl.page_size)?;
        Some(Memory {
            total_bytes,
            used_bytes,
        })
    });
    let disks = match (output(Source::DiskBytes), output(Source::DiskKilobytes)) {
        (Some(bytes), _) => df::disks(bytes, 1),
        (None, Some(kilobytes)) => df::disks(kilobytes, 1024),
        (None, None) => Vec::new(),
    };
    let interfaces = match (output(Source::NetDev), output(Source::Netstat)) {
        (Some(net_dev), _) => linux::interfaces(net_dev),
        (None, Some(netstat)) => macos::interfaces(netstat),
        (None, None) => Vec::new(),
    };
    Reading {
        cpu: output(Source::Stat)
            .and_then(linux::cpu)
            .or_else(|| output(Source::Top).and_then(macos::cpu)),
        memory: output(Source::Meminfo)
            .and_then(linux::memory)
            .or(mac_memory),
        load: output(Source::Loadavg)
            .and_then(linux::load_average)
            .or_else(|| sysctl.and_then(|sysctl| sysctl.load)),
        uptime_seconds: output(Source::Uptime)
            .and_then(linux::uptime_seconds)
            .or_else(|| sysctl.and_then(|sysctl| sysctl.uptime_seconds(at))),
        disks,
        interfaces,
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
