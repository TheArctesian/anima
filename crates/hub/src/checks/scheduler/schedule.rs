//! The scheduler's decisions: when a check first runs, when it is late, what a missed push
//! means and how a status change is reported (CHK-001, CHK-002, CHK-010). Pure: time and
//! randomness come in as parameters.

use std::collections::HashSet;

use chrono::{DateTime, TimeDelta, Utc};
use rand::Rng;

use crate::{
    checks::{config::CheckConfig, model::Due, probe::ProbeResult},
    events::Severity,
    ids::CheckId,
    status::Status,
};

fn interval(due: &Due) -> TimeDelta {
    TimeDelta::seconds(i64::from(due.settings.interval_seconds.max(1)))
}

/// When a check with no due time runs first. A probed check gets a uniform instant in
/// `[now, now + interval)`, so a hub restart does not probe every check at once (CHK-001).
/// A PUSH check waits its whole interval: its device has had no chance to call yet.
pub(super) fn first_due(due: &Due, now: DateTime<Utc>, rng: &mut impl Rng) -> DateTime<Utc> {
    let interval = interval(due);
    if matches!(due.config, Some(CheckConfig::Push)) {
        return now + interval;
    }
    let millis = interval.num_milliseconds();
    now + TimeDelta::milliseconds(rng.random_range(0..millis))
}

/// Whether the check's due time is more than 2 intervals before `now` (CHK-010). The caller
/// passes checks that are not paused.
pub(super) fn is_late(due: &Due, now: DateTime<Utc>) -> bool {
    due.state
        .next_due_at
        .is_some_and(|at| at < now - interval(due) * 2)
}

/// The late checks already reported, so each lateness episode gives one event (CHK-010).
#[derive(Debug, Default)]
pub(super) struct Watchdog {
    reported: HashSet<CheckId>,
}

impl Watchdog {
    /// Whether to report the check as late now: true once for each lateness episode.
    pub(super) fn should_report(&mut self, due: &Due, now: DateTime<Utc>) -> bool {
        is_late(due, now) && self.reported.insert(due.id)
    }

    /// The check ran, or its report was not written: its next lateness is reported again.
    pub(super) fn clear(&mut self, check: CheckId) {
        self.reported.remove(&check);
    }
}

/// A failed result with a fixed message, for a check that was not probed.
pub(super) fn failed(message: &str) -> ProbeResult {
    ProbeResult {
        ok: false,
        response_time: None,
        message: Some(message.to_owned()),
        resolved_ip: None,
        cert_not_after: None,
    }
}

/// A due PUSH check got no push within its interval: a failed attempt, so retries apply
/// through `evaluate` (CHK-002).
pub(super) fn missed_push() -> ProbeResult {
    failed("no push received")
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
