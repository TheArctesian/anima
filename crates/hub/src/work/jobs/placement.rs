//! Which job a machine gets (PRO-051, USE-003, USE-007): one pure function of the queue, of
//! what the machine said in its claim and of what the hub knows of the machine. The time is a
//! parameter (CQ-021).

use chrono::{DateTime, TimeDelta, Utc};
use daemon_core::wire::{Claim, Switch};

use super::model::{Job, Machine};
use crate::ids::DeviceId;

/// Jobs that a machine may refuse or lose in [`FAULT_WINDOW`] before the hub gives it no work
/// for [`OFF_FOR`] (K10 Section 11, SF-011).
pub(super) const MOST_FAULTS: usize = 5;
pub(super) const FAULT_WINDOW: TimeDelta = TimeDelta::hours(1);
pub(super) const OFF_FOR: TimeDelta = TimeDelta::hours(1);

/// Whether the hub gives `machine` no work at `now`: the operator set it off, or its hour with
/// no work has not ended (`WORK_OFF`).
pub(super) fn is_off(machine: &Machine, now: DateTime<Utc>) -> bool {
    machine.off_by_operator || machine.off_until.is_some_and(|until| now < until)
}

/// The end of the hour with no work for a machine with `faults` jobs refused or lost in the
/// last hour; `None` below the limit.
pub(super) fn off_until(faults: usize, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    (faults >= MOST_FAULTS).then(|| now + OFF_FOR)
}

/// Whether a claim can get a job at all: the switch at the machine is on, the machine is
/// clean and it has a free slot (K10 Section 4).
pub(super) fn can_take(claim: &Claim) -> bool {
    claim.switch == Switch::On && !claim.unclean && claim.free.slots >= 1
}

/// Whether the machine of `claim` can run `job`: the type, the kind, the operating system, the
/// architecture, the memory and the disk, and it did not refuse the job before.
pub(super) fn fits(job: &Job, device: DeviceId, claim: &Claim) -> bool {
    let asked = &job.asked;
    claim.types.contains(&asked.work_type)
        && claim.kinds.contains(&asked.kind)
        && asked.os.is_none_or(|os| os == claim.os)
        && asked
            .architecture
            .is_none_or(|architecture| architecture == claim.architecture)
        && asked.limits.memory_bytes <= claim.free.memory_bytes
        && asked.limits.disk_bytes <= claim.free.disk_bytes
        && !job.refused_by.contains(&device)
}

/// The job of `queued` (oldest first) that the machine gets: the oldest that fits and that this
/// machine did not hold last, or else the oldest that fits (USE-007: a job that a machine lost
/// goes to a different machine when one asks first).
pub(super) fn choose<'a>(queued: &'a [Job], device: DeviceId, claim: &Claim) -> Option<&'a Job> {
    if !can_take(claim) {
        return None;
    }
    let mut fitting = queued.iter().filter(|job| fits(job, device, claim));
    let first = fitting.next()?;
    if first.last_device != Some(device) {
        return Some(first);
    }
    fitting
        .find(|job| job.last_device != Some(device))
        .or(Some(first))
}
