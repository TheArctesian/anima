//! The activity of each drive between two readings: read and write rates and the busy share
//! (MET-028). Pure.

use std::time::Duration;

use super::{Kind, Sample};
use crate::readings::{DriveCounters, Reading};

/// The size `/proc/diskstats` counts sectors in, whatever the drive's own sector size.
const SECTOR_BYTES: u64 = 512;

/// The read, write and busy samples of each drive `current` lists, over the `elapsed` time
/// since `previous`. A drive without counters in both readings, or with a counter that went
/// back (a reboot, a device that came back), gets none, as an interface gets no rate.
pub(super) fn drive_activity(
    previous: &Reading,
    current: &Reading,
    elapsed: Duration,
) -> Vec<Sample> {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for drive in current.drives.iter().flatten() {
        let now = counters(&current.drive_counters, &drive.name);
        let was = counters(&previous.drive_counters, &drive.name);
        let Some((read, written, busy_ms)) = now.zip(was).and_then(|(now, was)| deltas(now, was))
        else {
            continue;
        };
        out.extend(Sample::new(Kind::DiskReadBps, &drive.name, read / seconds));
        out.extend(Sample::new(
            Kind::DiskWriteBps,
            &drive.name,
            written / seconds,
        ));
        let busy = busy_ms / (seconds * 1000.0) * 100.0;
        out.extend(Sample::new(Kind::DiskBusyPercent, &drive.name, busy));
    }
    out
}

/// The counters of the device named `name` (the first, if a name repeats).
fn counters<'a>(all: &'a [DriveCounters], name: &str) -> Option<&'a DriveCounters> {
    all.iter().find(|one| one.name == name)
}

/// Bytes read, bytes written and busy milliseconds from `was` to `now`; `None` when a counter
/// went back.
fn deltas(now: &DriveCounters, was: &DriveCounters) -> Option<(f64, f64, f64)> {
    let read = now.read_sectors.checked_sub(was.read_sectors)?;
    let written = now.written_sectors.checked_sub(was.written_sectors)?;
    let busy = now.busy_ms.checked_sub(was.busy_ms)?;
    Some((
        bytes(read.checked_mul(SECTOR_BYTES)?),
        bytes(written.checked_mul(SECTOR_BYTES)?),
        bytes(busy),
    ))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "counters above 2^53 lose their last digits, which a chart cannot show"
)]
const fn bytes(count: u64) -> f64 {
    count as f64
}
