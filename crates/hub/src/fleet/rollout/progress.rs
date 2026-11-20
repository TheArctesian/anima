//! What a rollout does next (ROL-026, ROL-027, ROL-039): one pure function of the rollout, of
//! its machines and of the time.

use chrono::{DateTime, TimeDelta, Utc};

use super::model::{Machine, Outcome, Reach};
use crate::{fleet::builds::Version, ids::DeviceId};

/// The waits of a rollout (ROL-026, ROL-047; OD-ROL-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pace {
    /// From the good first machine to the start of the next step.
    pub after_first: TimeDelta,
    /// From each other good step to the start of the next one.
    pub between: TimeDelta,
}

impl Pace {
    /// A rollout that the operator started with the press.
    pub const PRESSED: Self = Self {
        after_first: TimeDelta::minutes(10),
        between: TimeDelta::minutes(10),
    };
    /// A rollout that the hub started by itself (ROL-047).
    pub const AUTOMATIC: Self = Self {
        after_first: TimeDelta::hours(1),
        between: TimeDelta::minutes(10),
    };
}

/// The machines that update at the same time, with the times that the hub recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub machines: Vec<DeviceId>,
    pub started_at: Option<DateTime<Utc>>,
    /// When each machine of the step had passed its check or had left the step.
    pub good_at: Option<DateTime<Utc>>,
}

/// One rollout as the hub keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rollout {
    pub version: Version,
    pub steps: Vec<Step>,
    pub pace: Pace,
    /// The machines whose rollback or refusal the operator saw and continued after (ROL-027).
    pub continued_after: Vec<DeviceId>,
}

/// Where one machine of a started step is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// It passed its check (ROL-029), or it has a higher version.
    Passed,
    /// The step does not wait for it: it is not connected (ROL-039), or the hub tells it
    /// nothing (held, set off at the machine, no update function, token ended).
    Left,
    /// It updates, it checks, or it is busy and did not start (ROL-028).
    Pending,
    /// It rolled back or refused the release (ROL-027).
    Failed,
}

/// The standing of `machine` in a started step of a rollout to `version`. The outcome of a
/// try comes before the connection, as in the states of K8 Section 8.4.
#[must_use]
pub fn standing(machine: &Machine, version: Version) -> Standing {
    let Some(said) = machine.said.filter(|said| said.switch_on) else {
        return Standing::Left;
    };
    if machine.held || machine.reach == Reach::TokenEnded {
        return Standing::Left;
    }
    if machine
        .installed
        .is_some_and(|installed| installed > version)
    {
        return Standing::Passed;
    }
    match said
        .last
        .filter(|tried| tried.version == version)
        .map(|tried| tried.outcome)
    {
        Some(Outcome::Refused(_) | Outcome::RolledBack(_)) => Standing::Failed,
        Some(Outcome::Installed) if machine.installed == Some(version) => Standing::Passed,
        _ if machine.reach != Reach::Connected => Standing::Left,
        _ => Standing::Pending,
    }
}

/// What the hub does next for a rollout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Start the step with this index now.
    Start(usize),
    /// Record that the step with this index is good now; its wait starts.
    Good(usize),
    /// Stop: this machine rolled back or refused (ROL-027). No new step starts until the
    /// operator continues.
    Stop(DeviceId),
    /// Each step is good: the rollout is complete. A machine that left its step updates alone
    /// when it connects (ROL-040).
    Finish,
    /// Nothing now.
    Wait,
}

/// The next act for `rollout` at `now`. `machines` holds each machine of the rollout that the
/// hub still knows; a machine that is not in it left its step.
#[must_use]
pub fn next(rollout: &Rollout, machines: &[Machine], now: DateTime<Utc>) -> Next {
    let standing_of = |device: &DeviceId| {
        machines
            .iter()
            .find(|machine| machine.device == *device)
            .map_or(Standing::Left, |machine| standing(machine, rollout.version))
    };
    let started = rollout
        .steps
        .iter()
        .take_while(|step| step.started_at.is_some());
    let failed = started
        .clone()
        .flat_map(|step| &step.machines)
        .find(|device| {
            standing_of(device) == Standing::Failed && !rollout.continued_after.contains(device)
        });
    if let Some(device) = failed {
        return Next::Stop(*device);
    }
    let Some(index) = started.count().checked_sub(1) else {
        return if rollout.steps.is_empty() {
            Next::Finish
        } else {
            Next::Start(0)
        };
    };
    let step = &rollout.steps[index];
    let Some(good_at) = step.good_at else {
        let waits = step
            .machines
            .iter()
            .any(|device| standing_of(device) == Standing::Pending);
        return if waits { Next::Wait } else { Next::Good(index) };
    };
    if index + 1 == rollout.steps.len() {
        return Next::Finish;
    }
    let wait = if index == 0 {
        rollout.pace.after_first
    } else {
        rollout.pace.between
    };
    if now >= good_at + wait {
        Next::Start(index + 1)
    } else {
        Next::Wait
    }
}
