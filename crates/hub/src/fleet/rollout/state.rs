//! The update state of one machine, in the words of the client (K8 Section 8.4).

use super::model::{Cause, Machine, Outcome, Reach};
use crate::fleet::builds::Version;

/// The states of K8 Section 8.4, in the order of their tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    EnrolAgain,
    ManualInstallation,
    SetOff,
    Held,
    Refused(Cause),
    RolledBack(Cause),
    NotConnected,
    Updating,
    Checking,
    WaitingForStep,
    Available,
    UpToDate,
}

/// What the rollouts want for one machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Aimed {
    /// The version that the hub wants the machine to have now: that of the newest rollout that
    /// includes it and whose step for it started.
    pub now: Option<Version>,
    /// The version that the hub wanted before, when it wants none now or a different one.
    pub before: Option<Version>,
    /// True when a rollout includes the machine and its step did not start.
    pub waits_for_step: bool,
    /// The highest verified release that the hub holds for the target of the machine.
    pub newest: Option<Version>,
}

/// The state of `machine`: the first row of the table that is true.
#[must_use]
pub fn state(machine: &Machine, aimed: Aimed) -> State {
    if machine.reach == Reach::TokenEnded {
        return State::EnrolAgain;
    }
    let Some(said) = machine.said else {
        return State::ManualInstallation;
    };
    if !said.switch_on {
        return State::SetOff;
    }
    if machine.held {
        return State::Held;
    }
    let outcome_for = |version: Option<Version>| {
        said.last
            .filter(|tried| Some(tried.version) == version)
            .map(|tried| tried.outcome)
    };
    match (outcome_for(aimed.now), outcome_for(aimed.before)) {
        (Some(Outcome::Refused(cause)), _) => return State::Refused(cause),
        (Some(Outcome::RolledBack(cause)), _) | (_, Some(Outcome::RolledBack(cause))) => {
            return State::RolledBack(cause);
        }
        _ => {}
    }
    if aimed.now.is_some() {
        if machine.reach != Reach::Connected {
            return State::NotConnected;
        }
        if machine.installed != aimed.now {
            return State::Updating;
        }
        if outcome_for(aimed.now) != Some(Outcome::Installed) {
            return State::Checking;
        }
    }
    if aimed.waits_for_step {
        State::WaitingForStep
    } else if aimed.newest > machine.installed {
        State::Available
    } else {
        State::UpToDate
    }
}
