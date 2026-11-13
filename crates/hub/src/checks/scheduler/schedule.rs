//! The scheduler's decisions: when a check runs first, how soon it may run again, when it is
//! late and how a status change is reported (CHK-001, CHK-002, CHK-010, DB-005, R-102). Pure: time and randomness come in as
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

/// The earliest a check may be probed again, when its type has such a floor (R-102,
/// CHK-009): a domain expiry check asks its RDAP server at most once an interval, so it waits
/// its whole interval after `queried_at`, its last query. A check never queried, or of
/// another type, has none.
pub(super) fn floor(due: &Due, queried_at: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    let domain = matches!(due.config, Some(CheckConfig::DomainExpiry { .. }));
    queried_at
        .filter(|_| domain)
        .map(|at| at + interval(due.settings.interval_seconds))
}

/// `at`, or the check's [`floor`] after `queried_at` when that is later.
pub(super) fn not_before_floor(
    due: &Due,
    queried_at: Option<DateTime<Utc>>,
    at: DateTime<Utc>,
) -> DateTime<Utc> {
    floor(due, queried_at).map_or(at, |floor| at.max(floor))
}

/// Whether the check gets a new due time instead of running: it has none; it was due before
/// the scheduler started, so a hub restart does not probe everything at once; or it is due
/// before its [`floor`] after its last beat, however that due time was set (R-102).
pub(super) fn needs_new_due(due: &Due, started_at: DateTime<Utc>) -> bool {
    let floor = floor(due, due.last_beat_at);
    due.state
        .next_due_at
        .is_none_or(|at| at < started_at || floor.is_some_and(|floor| at < floor))
}

/// The new due time: spread, and never before the check's [`floor`] after its last beat. A
/// probed check gets a uniform instant in `[now, now + interval)` (CHK-001). A PUSH check
/// waits its whole interval: its device has had no chance to call.
pub(super) fn new_due(due: &Due, now: DateTime<Utc>, rng: &mut impl Rng) -> DateTime<Utc> {
    let interval = interval(due.settings.interval_seconds);
    if matches!(due.config, Some(CheckConfig::Push)) {
        return now + interval;
    }
    let millis = interval.num_milliseconds();
    let spread = now + TimeDelta::milliseconds(rng.random_range(0..millis));
    not_before_floor(due, due.last_beat_at, spread)
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

/// The event message for SNMP secrets removed because the device host changed: the check and
/// device names only, never a host or a secret (SEC-010, LOG-007).
pub(super) fn moved_secrets_message(check: &str, device: &str) -> String {
    format!(
        "Check {check} on {device} stopped: the device host changed after its SNMP secrets \
         were entered, so the hub removed them and sends nothing. Enter the community or the \
         v3 passwords again"
    )
}

/// The event message for a late check: the check and device names only (LOG-007).
pub(super) fn late_message(check: &str, device: &str) -> String {
    format!("Check {check} on {device} is late by more than 2 intervals")
}
