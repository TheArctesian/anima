//! The pure rules of a job (K10 Sections 4, 5 and 7): what a claim, a report, a lease that
//! ended, a stop and a time limit make of it. The time is a parameter (CQ-021).

use chrono::{DateTime, TimeDelta, Utc};
use daemon_core::wire::{MAX_LOG_PART_BYTES, StopReason, WorkReport, WorkState, WorkType};

use super::model::{Job, State};
use crate::{
    ids::DeviceId,
    lease::{self, Lost},
};

/// The reason of a `build` whose lease ended three times (K10 Section 7).
pub(super) const STOPPED_REASON: &str = "machine stopped 3 times";

/// The reason of a `runner` whose lease ended: a JIT configuration is for one use.
pub(super) const RUNNER_LOST_REASON: &str = "machine stopped; a runner job has one attempt";

/// The reason of an attempt that the hub ended at its time limit.
pub(super) const EXPIRED_REASON: &str = "expired";

/// How long after `limits.seconds` the hub ends an attempt that is not final (SF-011).
const OVERTIME: TimeDelta = TimeDelta::minutes(5);

/// Why a report is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    /// Not the holder, a different attempt, a job that ended, a lease that ended, or a change
    /// that the diagram does not permit: `NOT_HOLDER`.
    NotHolder,
    /// A result with a state that is not final, or a final state with no result: `MALFORMED`.
    Malformed,
}

/// The job that `device` claimed at `now`: the next attempt, with its lease.
pub(super) fn claimed(job: &Job, device: DeviceId, now: DateTime<Utc>) -> Job {
    Job {
        state: State::Claimed,
        attempt: job.attempt.saturating_add(1),
        holder: Some(device),
        last_device: Some(device),
        lease_ends_at: Some(lease::end(now)),
        claimed_at: Some(now),
        stop: None,
        changed_at: now,
        ..job.clone()
    }
}

/// Whether the diagram of K10 Section 7 permits `from` to `to`. The same state is a report
/// that only holds the lease.
const fn permitted(from: State, to: WorkState) -> bool {
    use State::{Claimed, Finishing, Preparing, Running};
    matches!(
        (from, to),
        (
            Claimed,
            WorkState::Claimed | WorkState::Preparing | WorkState::Refused
        ) | (Preparing, WorkState::Preparing | WorkState::Running)
            | (Running, WorkState::Running | WorkState::Finishing)
            | (Finishing, WorkState::Finishing | WorkState::Done)
            | (
                Preparing | Running | Finishing,
                WorkState::Failed | WorkState::Stopped
            )
    )
}

/// The job after the report of `device` at `now` (K10 Section 7). The log part is not applied
/// here: see [`log_fits`].
pub(super) fn reported(
    job: &Job,
    device: DeviceId,
    report: &WorkReport,
    now: DateTime<Utc>,
) -> Result<Job, Refusal> {
    let holds = job.holder == Some(device)
        && job.attempt == report.attempt
        && job.state.is_held()
        && !lease::ended(job.lease_ends_at, now);
    if !holds || !permitted(job.state, report.state) {
        return Err(Refusal::NotHolder);
    }
    if report.state.is_final() != report.result.is_some() {
        return Err(Refusal::Malformed);
    }
    let Some(state) = State::reported(report.state) else {
        return Ok(refused(job, device, now));
    };
    let mut next = Job {
        state,
        changed_at: now,
        ..job.clone()
    };
    if state.is_final() {
        release(&mut next);
        next.reason = report
            .result
            .as_ref()
            .and_then(|result| result.cause.clone());
        next.result.clone_from(&report.result);
        next.ended_at = Some(now);
    } else {
        next.lease_ends_at = Some(lease::end(now));
    }
    Ok(next)
}

/// Takes the job from its holder.
fn release(job: &mut Job) {
    job.holder = None;
    job.lease_ends_at = None;
    job.stop = None;
}

/// The job that `device` refused: back in the queue, and not for that machine again. A refusal
/// is not an attempt that was lost (K10 Section 5).
fn refused(job: &Job, device: DeviceId, now: DateTime<Utc>) -> Job {
    let mut next = queued_again(job, now);
    if !next.refused_by.contains(&device) {
        next.refused_by.push(device);
    }
    next
}

fn queued_again(job: &Job, now: DateTime<Utc>) -> Job {
    let mut next = Job {
        state: State::Queued,
        claimed_at: None,
        changed_at: now,
        ..job.clone()
    };
    release(&mut next);
    next
}

fn failed(job: &Job, reason: &str, now: DateTime<Utc>) -> Job {
    let mut next = Job {
        state: State::Failed,
        reason: Some(reason.to_owned()),
        ended_at: Some(now),
        changed_at: now,
        ..job.clone()
    };
    release(&mut next);
    next
}

/// The job after its lease ended at `now` with no report (K10 Section 7). A `build` goes back
/// to the queue and fails the third time; a `runner` has no second attempt.
pub(super) fn lease_lost(job: &Job, now: DateTime<Utc>) -> Job {
    if job.asked.work_type == WorkType::Runner {
        return failed(job, RUNNER_LOST_REASON, now);
    }
    match lease::lost(job.lost) {
        Lost::Again(lost) => Job {
            lost,
            ..queued_again(job, now)
        },
        Lost::Failed(lost) => Job {
            lost,
            ..failed(job, STOPPED_REASON, now)
        },
    }
}

/// The job whose holder does not name it in a claim: the answer to its claim was lost. It goes
/// back to the queue and no lease counts as lost (SF-011).
pub(super) fn answer_lost(job: &Job, now: DateTime<Utc>) -> Job {
    queued_again(job, now)
}

/// Whether the attempt holds the job for longer than its limit and 5 minutes (SF-011).
pub(super) fn overdue(job: &Job, now: DateTime<Utc>) -> bool {
    let limit = i64::try_from(job.asked.limits.seconds).unwrap_or(i64::MAX);
    let most = limit.saturating_add(OVERTIME.num_seconds());
    job.state.is_held()
        && job
            .claimed_at
            .is_some_and(|since| now.signed_duration_since(since).num_seconds() >= most)
}

/// The job that the hub ended at its time limit.
pub(super) fn expired(job: &Job, now: DateTime<Utc>) -> Job {
    let mut next = Job {
        state: State::Stopped,
        reason: Some(EXPIRED_REASON.to_owned()),
        ended_at: Some(now),
        changed_at: now,
        ..job.clone()
    };
    release(&mut next);
    next
}

/// The job after a stop was asked for at `now`. A queued job stops at once. A held job gets
/// the stop in the next answer to its holder; the first reason stays. A job that ended is `None`.
pub(super) fn stop_asked(job: &Job, reason: StopReason, now: DateTime<Utc>) -> Option<Job> {
    match job.state {
        State::Queued => Some(Job {
            state: State::Stopped,
            reason: Some(reason.as_str().to_owned()),
            ended_at: Some(now),
            changed_at: now,
            ..job.clone()
        }),
        state if state.is_held() => Some(Job {
            stop: job.stop.or(Some(reason)),
            ..job.clone()
        }),
        _ => None,
    }
}

/// Whether a log part that starts at `from` with `bytes` bytes is the next part of the log of
/// the job. A part that is not is dropped, and the answer names the end that the hub holds.
pub(super) fn log_fits(job: &Job, from: u64, bytes: usize) -> bool {
    from == job.log_end && bytes <= MAX_LOG_PART_BYTES
}
