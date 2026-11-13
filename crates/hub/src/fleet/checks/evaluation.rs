//! What one probe result means for a check: retries, invert and the Degraded limit
//! (CHK-002, CHK-008). Pure: time comes in as a parameter.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};

use super::probe::{Judgement, ProbeResult};
use crate::status::Status;

/// The common settings of a check (spec 6.4), with the contract defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckSettings {
    pub interval_seconds: u32,
    /// Failed retries after the first failed attempt before the check is Down (CHK-002).
    pub retries: u8,
    pub retry_interval_seconds: u32,
    /// A success slower than this many milliseconds is Degraded.
    pub degraded_threshold_ms: u32,
    /// A failed probe counts as a success and a success as a failure (CHK-008).
    pub invert: bool,
}

impl Default for CheckSettings {
    fn default() -> Self {
        Self {
            interval_seconds: 60,
            retries: 3,
            retry_interval_seconds: 20,
            degraded_threshold_ms: 1000,
            invert: false,
        }
    }
}

/// What the scheduler keeps for a check between probes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckState {
    /// The status the check shows: Unknown before its first result.
    pub status: Status,
    /// Failed attempts in a row.
    pub failures: u32,
    /// When the next probe is due; `None` means as soon as possible.
    pub next_due_at: Option<DateTime<Utc>>,
}

impl Default for CheckState {
    fn default() -> Self {
        Self {
            status: Status::Unknown,
            failures: 0,
            next_due_at: None,
        }
    }
}

fn after(now: DateTime<Utc>, seconds: u32) -> DateTime<Utc> {
    now + TimeDelta::seconds(i64::from(seconds))
}

/// The check's new state, and the status to record in the heartbeat.
///
/// Invert is applied first (CHK-008); an inverted success has no response time, so it is
/// never Degraded. A success is Degraded when slower than the threshold, else Up, and resets
/// the failures; the next probe is due after the interval.
///
/// A result the probe could not judge ([`Judgement::Unjudged`]) is recorded as Unknown with
/// its message: failures reset, the next probe due after the interval, invert not applied.
/// A [`Judgement::Starting`] or [`Judgement::Expiring`] success is Degraded; inverted, it is
/// a failure (D-2, D-29).
///
/// A failure is Down only when `failures > retries`: the first attempt plus `retries` failed
/// retries (CHK-002). Until then the check keeps its previous status, the next probe is due
/// after the retry interval, and the heartbeat records Unknown (pending), because the
/// outcome is not settled yet.
#[must_use]
pub fn evaluate(
    prev: &CheckState,
    probe: &ProbeResult,
    cfg: &CheckSettings,
    now: DateTime<Utc>,
) -> (CheckState, Status) {
    if probe.judgement == Judgement::Unjudged {
        let state = CheckState {
            status: Status::Unknown,
            failures: 0,
            next_due_at: Some(after(now, cfg.interval_seconds)),
        };
        return (state, Status::Unknown);
    }
    let succeeded = probe.ok != cfg.invert;
    if succeeded {
        let response_time = if cfg.invert {
            None
        } else {
            probe.response_time
        };
        let limit = Duration::from_millis(u64::from(cfg.degraded_threshold_ms));
        let warned = matches!(probe.judgement, Judgement::Starting | Judgement::Expiring);
        let status = if warned || response_time.is_some_and(|time| time > limit) {
            Status::Degraded
        } else {
            Status::Up
        };
        let state = CheckState {
            status,
            failures: 0,
            next_due_at: Some(after(now, cfg.interval_seconds)),
        };
        return (state, status);
    }
    let failures = prev.failures.saturating_add(1);
    if failures > u32::from(cfg.retries) {
        let state = CheckState {
            status: Status::Down,
            failures,
            next_due_at: Some(after(now, cfg.interval_seconds)),
        };
        return (state, Status::Down);
    }
    let state = CheckState {
        status: prev.status,
        failures,
        next_due_at: Some(after(now, cfg.retry_interval_seconds)),
    };
    (state, Status::Unknown)
}
