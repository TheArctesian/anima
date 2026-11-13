//! When a device's collections start failing and when they work again, as one event each, not
//! one every round (D-10). Pure.
//!
//! The messages are fixed text: no host, address, command output or secret (LOG-007). The
//! reason of a failure goes to the process log with the device id only.

use crate::{
    events::{ActorKind, NewEvent, Severity, Source},
    ids::DeviceId,
};

pub(super) const FAILING_MESSAGE: &str =
    "Metrics cannot be collected from the device; the hub tries again every 30 seconds";
pub(super) const WORKING_MESSAGE: &str = "Metrics are collected from the device again";

/// A change worth one event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Change {
    /// The device's collection failed after working, or failed first after the hub started.
    StartedFailing,
    /// The device's collection worked after failing.
    WorksAgain,
}

/// The change, if any, when a device that `was_failing` has a collection that `collected` or
/// not. A repeat of the same outcome is no change.
pub(super) const fn change(was_failing: bool, collected: bool) -> Option<Change> {
    match (was_failing, collected) {
        (false, false) => Some(Change::StartedFailing),
        (true, true) => Some(Change::WorksAgain),
        (false, true) | (true, false) => None,
    }
}

/// The event that reports `change` for `device` and its name. `events` stores the device
/// under `Source::Metrics` (H-1).
pub(super) fn event(change: Change, device: DeviceId, name: String) -> NewEvent {
    let (severity, message) = match change {
        Change::StartedFailing => (Severity::Warning, FAILING_MESSAGE),
        Change::WorksAgain => (Severity::Info, WORKING_MESSAGE),
    };
    NewEvent {
        severity,
        source: Source::Metrics,
        device: Some((device, name)),
        message: message.to_owned(),
        actor: ActorKind::Hub,
    }
}
