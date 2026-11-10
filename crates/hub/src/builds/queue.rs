//! The pure job rules (BLD-009): which state changes are permitted, the lease and its end, what
//! a builder may claim, and the job each change leaves. Time comes in as a parameter (CQ-021).

use chrono::{DateTime, TimeDelta, Utc};

use super::{
    log_lines,
    model::{Arch, BuildJob, Builder, JobState, Os, Platform, Report, Target},
};
use crate::ids::BuilderId;

/// How long a claim or a report holds the job (BLD-005).
pub(super) const LEASE: TimeDelta = TimeDelta::seconds(90);

/// A builder that reported within this time is online (BLD-007).
pub(super) const ONLINE_WITHIN: TimeDelta = TimeDelta::seconds(60);

/// Leases that may end without a report before the job fails (BLD-006).
const MAX_ATTEMPTS: u8 = 3;

/// The reason of a job whose lease ended [`MAX_ATTEMPTS`] times (BLD-006).
pub(super) const STOPPED_REASON: &str = "builder stopped 3 times";

/// Why a change of a job is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    /// The builder does not hold the job's lease (BLD-010).
    NotHolder,
    /// The lease ended before this report; the job goes back to the queue (BLD-006).
    LeaseEnded,
    /// The diagram does not permit this change (BLD-010), or the operator may not cancel a job
    /// that left the queue (BLD-008).
    Change { from: JobState, to: JobState },
}

/// Whether the diagram of Section 3.10.1 permits `from` to `to`.
pub(super) const fn permitted(from: JobState, to: JobState) -> bool {
    use JobState::{Building, Cancelled, Claimed, Failed, Published, Queued, Uploading};
    matches!(
        (from, to),
        (Queued, Claimed | Cancelled)
            | (Claimed, Building | Failed | Queued)
            | (Building, Uploading | Failed | Queued)
            | (Uploading, Published | Failed | Queued)
    )
}

/// The lease end of a claim or report at `now` (BLD-005).
pub(super) fn lease_end(now: DateTime<Utc>) -> DateTime<Utc> {
    now + LEASE
}

/// Whether the job's lease has ended at `now`. A job without a lease has none to end.
pub(super) fn lease_ended(job: &BuildJob, now: DateTime<Utc>) -> bool {
    job.state.is_leased() && job.lease_ends_at.is_none_or(|end| now >= end)
}

/// Whether a builder on `platform` can build `target` (BLD-003).
pub(super) const fn can_build(platform: Platform, target: Target) -> bool {
    matches!(
        (target, platform.os, platform.arch),
        (Target::LinuxX86_64, Os::Linux, Arch::X86_64)
            | (Target::DarwinUniversal, Os::Macos, Arch::Aarch64)
    )
}

/// The targets a builder on `platform` may claim.
pub(super) fn buildable(platform: Platform) -> Vec<Target> {
    Target::ALL
        .into_iter()
        .filter(|target| can_build(platform, *target))
        .collect()
}

/// Whether a builder seen last at `last_seen` is online at `now` (BLD-007).
pub(super) fn online(last_seen: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    last_seen.is_some_and(|seen| now - seen <= ONLINE_WITHIN)
}

/// The job in state `to` at `now`, with the time of that state recorded.
fn entered(job: &BuildJob, to: JobState, now: DateTime<Utc>) -> BuildJob {
    let mut next = job.clone();
    next.state = to;
    next.times.changed = now;
    let slot = match to {
        JobState::Queued => {
            next.times.queued = now;
            return next;
        }
        JobState::Claimed => &mut next.times.claimed,
        JobState::Building => &mut next.times.building,
        JobState::Uploading => &mut next.times.uploading,
        JobState::Published => &mut next.times.published,
        JobState::Failed => &mut next.times.failed,
        JobState::Cancelled => &mut next.times.cancelled,
    };
    *slot = Some(now);
    next
}

/// The queued job claimed by `builder` at `now`, with a fresh lease (BLD-004, BLD-005).
pub(super) fn claim(job: &BuildJob, builder: &Builder, now: DateTime<Utc>) -> BuildJob {
    let mut next = entered(job, JobState::Claimed, now);
    next.builder = Some(builder.id);
    next.builder_name = Some(builder.name.clone());
    next.lease_ends_at = Some(lease_end(now));
    next
}

/// The job after its holder's report at `now`: in the reported state when given (a report of
/// the current state only extends the lease), with the lease extended, or ended for `failed`
/// with the cleaned reason (BLD-005, BLD-010). A builder may report only `building`, `uploading` and
/// `failed`; publishing is the hub's (BLD-066).
pub(super) fn report(
    job: &BuildJob,
    holder: BuilderId,
    report: &Report,
    now: DateTime<Utc>,
) -> Result<BuildJob, Refusal> {
    if job.builder != Some(holder) || !job.state.is_leased() {
        return Err(Refusal::NotHolder);
    }
    if lease_ended(job, now) {
        return Err(Refusal::LeaseEnded);
    }
    let to = report.state.unwrap_or(job.state);
    let reportable = matches!(
        to,
        JobState::Building | JobState::Uploading | JobState::Failed
    );
    let mut next = if to == job.state {
        job.clone()
    } else if reportable && permitted(job.state, to) {
        entered(job, to, now)
    } else {
        return Err(Refusal::Change {
            from: job.state,
            to,
        });
    };
    if to == JobState::Failed {
        next.lease_ends_at = None;
        next.failure_reason = Some(log_lines::reason(report.reason.as_deref()));
    } else {
        next.lease_ends_at = Some(lease_end(now));
    }
    Ok(next)
}

/// The job after its lease ended at `now`: back in the queue with one more attempt, or
/// `failed` with [`STOPPED_REASON`] on the third (BLD-006). The builder is released.
pub(super) fn expire(job: &BuildJob, now: DateTime<Utc>) -> BuildJob {
    let attempts = job.attempts.saturating_add(1).min(MAX_ATTEMPTS);
    let mut next = if attempts >= MAX_ATTEMPTS {
        let mut failed = entered(job, JobState::Failed, now);
        failed.failure_reason = Some(STOPPED_REASON.to_owned());
        failed
    } else {
        entered(job, JobState::Queued, now)
    };
    next.attempts = attempts;
    next.builder = None;
    next.lease_ends_at = None;
    next
}

/// The job cancelled by the operator at `now`; only a queued job can be (BLD-008).
pub(super) fn cancel(job: &BuildJob, now: DateTime<Utc>) -> Result<BuildJob, Refusal> {
    if job.state == JobState::Queued {
        Ok(entered(job, JobState::Cancelled, now))
    } else {
        Err(Refusal::Change {
            from: job.state,
            to: JobState::Cancelled,
        })
    }
}
