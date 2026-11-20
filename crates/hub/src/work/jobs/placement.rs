//! Which job a machine gets, and why (PRO-051, USE-003, USE-007; ROU-004 to ROU-007, R-259):
//! pure functions of the queue, of what the machine said in its claim and of what the hub
//! knows of the machines. The time is a parameter (CQ-021).

use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use daemon_core::wire::{Claim, Switch, WorkArchitecture, WorkKind, WorkOs, WorkType};

use super::model::{Job, Machine};
use crate::ids::{DeviceId, ProjectId};

/// Jobs that a machine may refuse or lose in [`FAULT_WINDOW`] before the hub gives it no work
/// for [`OFF_FOR`] (K10 Section 11, SF-011).
pub(super) const MOST_FAULTS: usize = 5;
pub(super) const FAULT_WINDOW: TimeDelta = TimeDelta::hours(1);
pub(super) const OFF_FOR: TimeDelta = TimeDelta::hours(1);

/// How long a new job in the queue waits for a machine with more free memory (R-259).
pub(super) const HOLD: TimeDelta = TimeDelta::seconds(20);
/// The most age of the last claim of a machine that a job can wait for.
pub(super) const FRESH: TimeDelta = TimeDelta::seconds(30);

/// What a machine said that it is and has, in a claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub device: DeviceId,
    pub types: Vec<WorkType>,
    pub kinds: Vec<WorkKind>,
    /// `None` for a word that this hub does not know: it fits no job that asks for a system.
    pub os: Option<WorkOs>,
    pub architecture: Option<WorkArchitecture>,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub excluded: bool,
}

impl Offer {
    pub(super) fn of(claim: &Claim, machine: &Machine) -> Self {
        Self {
            device: machine.device,
            types: claim.types.clone(),
            kinds: claim.kinds.clone(),
            os: Some(claim.os),
            architecture: Some(claim.architecture),
            memory_bytes: claim.free.memory_bytes,
            disk_bytes: claim.free.disk_bytes,
            excluded: machine.excluded,
        }
    }
}

/// The decision of the placement for one job and one machine: it fits, or the one requirement
/// that failed first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Fits,
    Type,
    Kind,
    Os,
    Architecture,
    Memory,
    Disk,
    /// The machine refused this job before (K10 Section 5).
    RefusedBefore,
    /// The job, or the pin of its project, names a different machine (ROU-006).
    NamesOther,
    /// The operator excluded the machine, and the job names no machine (ROU-006).
    Excluded,
    /// The machine fits, but a machine with more free memory asked a short time ago (R-259).
    WaitsForBetter,
}

impl Verdict {
    pub(super) const ALL: [Self; 11] = [
        Self::Fits,
        Self::Type,
        Self::Kind,
        Self::Os,
        Self::Architecture,
        Self::Memory,
        Self::Disk,
        Self::RefusedBefore,
        Self::NamesOther,
        Self::Excluded,
        Self::WaitsForBetter,
    ];

    /// The verdict as the database and the audit entries spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fits => "fits",
            Self::Type => "type",
            Self::Kind => "kind",
            Self::Os => "os",
            Self::Architecture => "architecture",
            Self::Memory => "memory",
            Self::Disk => "disk",
            Self::RefusedBefore => "refused_before",
            Self::NamesOther => "names_other",
            Self::Excluded => "excluded",
            Self::WaitsForBetter => "waits_for_better",
        }
    }

    pub(super) fn from_stored(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|each| each.as_str() == text)
    }
}

/// What a claim is decided against, but the queue and the machine that asks.
pub(super) struct Around<'a> {
    /// The machine that the operator pinned for a project.
    pub pins: &'a HashMap<ProjectId, DeviceId>,
    /// The other machines that asked for work a short time ago and can take a job.
    pub others: &'a [Offer],
    pub now: DateTime<Utc>,
}

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

/// The one machine that may get `job`: the machine that the job names, or else the machine
/// that the operator pinned for its project (ROU-006).
pub(super) fn named(job: &Job, pins: &HashMap<ProjectId, DeviceId>) -> Option<DeviceId> {
    let pinned = || job.asked.project.and_then(|project| pins.get(&project));
    job.asked.machine.or_else(|| pinned().copied())
}

/// Whether the machine of `offer` can run `job`, with no look at a different machine: the name
/// or the exclusion, the type, the kind, the operating system, the architecture, the memory and
/// the disk, and that it did not refuse the job before.
pub(super) fn alone(job: &Job, named: Option<DeviceId>, offer: &Offer) -> Verdict {
    let asked = &job.asked;
    let checks = [
        (
            named.is_none_or(|device| device == offer.device),
            Verdict::NamesOther,
        ),
        (named.is_some() || !offer.excluded, Verdict::Excluded),
        (offer.types.contains(&asked.work_type), Verdict::Type),
        (offer.kinds.contains(&asked.kind), Verdict::Kind),
        (asked.os.is_none_or(|os| Some(os) == offer.os), Verdict::Os),
        (
            asked
                .architecture
                .is_none_or(|architecture| Some(architecture) == offer.architecture),
            Verdict::Architecture,
        ),
        (
            asked.limits.memory_bytes <= offer.memory_bytes,
            Verdict::Memory,
        ),
        (asked.limits.disk_bytes <= offer.disk_bytes, Verdict::Disk),
        (
            !job.refused_by.contains(&offer.device),
            Verdict::RefusedBefore,
        ),
    ];
    checks
        .into_iter()
        .find_map(|(holds, verdict)| (!holds).then_some(verdict))
        .unwrap_or(Verdict::Fits)
}

/// The decision for `job` and the machine of `offer`. The free memory is a tie-break only: for
/// [`HOLD`] after the job came into the queue, a machine that fits does not get it when a
/// different machine that fits said that it has more free memory (R-259, ROU-002).
pub(super) fn verdict(job: &Job, offer: &Offer, around: &Around<'_>) -> Verdict {
    let named = named(job, around.pins);
    let own = alone(job, named, offer);
    let new = around.now - job.changed_at < HOLD;
    let better = |other: &Offer| {
        other.device != offer.device
            && other.memory_bytes > offer.memory_bytes
            && alone(job, named, other) == Verdict::Fits
    };
    if own == Verdict::Fits && new && around.others.iter().any(better) {
        return Verdict::WaitsForBetter;
    }
    own
}

/// The job of `queued` (oldest first) that the machine gets, by the verdict of each: the oldest
/// that fits and that this machine did not hold last, or else the oldest that fits (USE-007: a
/// job that a machine lost goes to a different machine when one asks first).
pub(super) fn choose<'a>(
    queued: &'a [Job],
    verdicts: &[Verdict],
    device: DeviceId,
) -> Option<&'a Job> {
    let mut fitting = queued
        .iter()
        .zip(verdicts)
        .filter_map(|(job, verdict)| (*verdict == Verdict::Fits).then_some(job));
    let first = fitting.next()?;
    if first.last_device != Some(device) {
        return Some(first);
    }
    fitting
        .find(|job| job.last_device != Some(device))
        .or(Some(first))
}

/// Why the machine that a new job names can never run it, from what its last claim said; `None`
/// when it can, or when only its free memory or disk is too small now, which can change
/// (ROU-006, JOB-014). `known` is `None` for a machine that never asked this hub for work.
pub(super) fn never_fits(
    job: &Job,
    known: Option<(&Machine, &Offer)>,
    now: DateTime<Utc>,
) -> Option<Unfit> {
    let Some((machine, offer)) = known else {
        return Some(Unfit::NeverAsked);
    };
    if machine.switch == Switch::Off || is_off(machine, now) {
        return Some(Unfit::TakesNoWork);
    }
    let fixed = [
        Verdict::Type,
        Verdict::Kind,
        Verdict::Os,
        Verdict::Architecture,
    ];
    let found = alone(job, Some(offer.device), offer);
    fixed.contains(&found).then_some(Unfit::Requirement(found))
}

/// Why a named machine can never run a job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unfit {
    /// The machine never asked this hub for work.
    NeverAsked,
    /// The switch at the machine or in the hub is off.
    TakesNoWork,
    Requirement(Verdict),
}
