//! The order of a rollout (ROL-023, ROL-025): one machine first, then the other machines in
//! steps. The hub itself is not part of a rollout (stage C is deferred, R-139).

use super::model::{Machine, Reach};
use crate::ids::DeviceId;

/// The most machines of one step for a rollout of `machines` machines: a quarter, and at least
/// one (ROL-023).
#[must_use]
pub const fn step_size(machines: usize) -> usize {
    let quarter = machines / 4;
    if quarter == 0 { 1 } else { quarter }
}

/// The machine that the policy selects as the first of a rollout (ROL-025): the first of
/// `included`, in the order given, that is connected, that is not the machine of the hub and
/// that runs no job. `None` when no machine is such: the operator then selects one.
#[must_use]
pub fn first_machine(included: &[Machine]) -> Option<DeviceId> {
    included
        .iter()
        .find(|machine| {
            machine.reach == Reach::Connected
                && !machine.with_hub
                && machine.said.is_some_and(|said| !said.busy)
        })
        .map(|machine| machine.device)
}

/// The steps of a rollout: `first` alone, then `others` in their order, in steps of at most a
/// quarter of all the machines (ROL-023). `first` is the choice of the policy or of the
/// operator; a second mention of it in `others` is left out.
#[must_use]
pub fn steps(first: DeviceId, others: &[DeviceId]) -> Vec<Vec<DeviceId>> {
    let rest: Vec<DeviceId> = others
        .iter()
        .copied()
        .filter(|other| *other != first)
        .collect();
    let size = step_size(rest.len() + 1);
    let mut all = vec![vec![first]];
    all.extend(rest.chunks(size).map(<[DeviceId]>::to_vec));
    all
}
