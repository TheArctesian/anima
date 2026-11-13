//! What the windows in force at one instant cover (MW-003).

use std::collections::HashSet;

use chrono::{DateTime, Utc};

use super::model::{Target, Window};
use crate::ids::{CheckId, DeviceId};

/// The devices, checks and tags that active windows apply to, at one instant. Build it once
/// and ask it about as many devices as needed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActiveTargets {
    devices: HashSet<DeviceId>,
    checks: HashSet<CheckId>,
    tags: HashSet<String>,
}

impl ActiveTargets {
    /// The targets of the windows that are in force at `now`.
    pub(super) fn of(windows: &[Window], now: DateTime<Utc>) -> Self {
        let mut active = Self::default();
        for target in windows
            .iter()
            .filter(|window| window.is_active(now))
            .flat_map(|window| &window.targets)
        {
            match target {
                Target::Device(id) => active.devices.insert(*id),
                Target::Check(id) => active.checks.insert(*id),
                Target::Tag(tag) => active.tags.insert(tag.clone()),
            };
        }
        active
    }

    /// Whether an active window applies to the device, to the check (when one is asked
    /// about), or to any of the tags. A check target applies only to that check, never to
    /// its whole device.
    #[must_use]
    pub fn covers(&self, device: DeviceId, check: Option<CheckId>, tags: &[String]) -> bool {
        self.devices.contains(&device)
            || check.is_some_and(|check| self.checks.contains(&check))
            || tags.iter().any(|tag| self.tags.contains(tag))
    }
}
