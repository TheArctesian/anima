//! The scheduler's decisions: when a check runs first, when it is late and how a status change
//! is reported (CHK-001, CHK-002, CHK-010, DB-005). Pure: time and randomness come in as
//! parameters.

use std::collections::HashSet;

use chrono::{DateTime, TimeDelta, Utc};
use rand::{Rng, RngExt as _};

use crate::{
    checks::{
        config::CheckConfig,
        model::Due,
        probe::{Failure, ProbeResult},
    },
    events::Severity,
    ids::CheckId,
    status::Status,
};

fn interval(seconds: u32) -> TimeDelta {
    TimeDelta::seconds(i64::from(seconds.max(1)))
}

/// Whether the check gets a new, spread due time instead of running: it has none, or it was
/// due before the scheduler started, so a hub restart does not probe everything at once.
pub(super) fn needs_spreading(due: &Due, started_at: DateTime<Utc>) -> bool {
    due.state.next_due_at.is_none_or(|at| at < started_at)
}

/// The spread due time. A probed check gets a uniform instant in `[now, now + interval)`
/// (CHK-001). A PUSH check waits its whole interval: its device has had no chance to call.
pub(super) fn first_due(due: &Due, now: DateTime<Utc>, rng: &mut impl Rng) -> DateTime<Utc> {
    let interval = interval(due.settings.interval_seconds);
    if matches!(due.config, Some(CheckConfig::Push)) {
        return now + interval;
    }
    let millis = interval.num_milliseconds();
    now + TimeDelta::milliseconds(rng.random_range(0..millis))
}

/// When a check is due and how often it runs, for the lateness rule.
#[derive(Clone, Copy, Debug)]
pub(super) struct Timing {
    pub(super) next_due_at: Option<DateTime<Utc>>,
    pub(super) interval_seconds: u32,
}

impl From<&Due> for Timing {
    fn from(due: &Due) -> Self {
        Self {
            next_due_at: due.state.next_due_at,
            interval_seconds: due.settings.interval_seconds,
        }
    }
}

/// Whether a check that is not paused is late by more than 2 intervals at `now` (CHK-010).
/// Lateness counts from the due time, or from `started_at` when the check was due before the
/// scheduler started, so the downtime of the hub itself is not reported.
pub(super) fn is_late(timing: Timing, started_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    timing
        .next_due_at
        .is_some_and(|at| at.max(started_at) < now - interval(timing.interval_seconds) * 2)
}

/// The late checks already reported, so each lateness episode gives one event (CHK-010).
#[derive(Debug, Default)]
pub(super) struct Watchdog {
    reported: HashSet<CheckId>,
}

impl Watchdog {
    /// Whether to report the check as late now: true once for each lateness episode.
    pub(super) fn should_report(&mut self, check: CheckId, late: bool) -> bool {
        late && self.reported.insert(check)
    }

    /// The check ran on time again, or its report was not written: its next lateness is
    /// reported again.
    pub(super) fn clear(&mut self, check: CheckId) {
        self.reported.remove(&check);
    }

    /// Forgets the checks that are not in `kept`.
    pub(super) fn retain(&mut self, kept: &HashSet<CheckId>) {
        self.reported.retain(|check| kept.contains(check));
    }
}

/// A due PUSH check got no push within its interval: a failed attempt, so retries apply
/// through `evaluate` (CHK-002).
pub(super) fn missed_push() -> ProbeResult {
    ProbeResult::failed(&Failure::NoPush)
}

/// The severity of the event for a device that changed to `status`.
pub(super) const fn severity_of(status: Status) -> Severity {
    match status {
        Status::Down => Severity::Error,
        Status::Degraded => Severity::Warning,
        Status::Up | Status::Maintenance | Status::Unknown => Severity::Info,
    }
}

const fn word(status: Status) -> &'static str {
    match status {
        Status::Up => "Up",
        Status::Down => "Down",
        Status::Degraded => "Degraded",
        Status::Maintenance => "in maintenance",
        Status::Unknown => "Unknown",
    }
}

/// The event message for a status change: the device name and the status only (LOG-007).
pub(super) fn status_message(device: &str, status: Status) -> String {
    format!("{device} is {}", word(status))
}

/// The event message for a late check: the check and device names only (LOG-007).
pub(super) fn late_message(check: &str, device: &str) -> String {
    format!("Check {check} on {device} is late by more than 2 intervals")
}
