//! The batteries of a batch (K8 Section 4; MET-032, CAP-004): their wire form and the hub's
//! check of them.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::points::is_label;
use crate::readings::{self, Battery, batteries::MAX_BATTERIES};

/// The longest time left a batch may carry, in seconds (K8 Section 4).
const MAX_SECONDS_LEFT: u32 = i32::MAX.unsigned_abs();

/// A battery that powers the device. Its charge is the series `battery_percent` with its name
/// as the label, not a member here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchBattery {
    pub name: String,
    pub state: BatteryState,
    /// The time to empty while it discharges or to full while it charges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds_left: Option<u32>,
    /// The capacity when full against the capacity of its design, 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_percent: Option<u8>,
}

/// What a battery is doing, as a batch spells it. A state a newer daemon knows and this hub
/// does not reads as `unknown`, so the batch is not refused for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    NotCharging,
    #[serde(other)]
    Unknown,
}

impl From<readings::BatteryState> for BatteryState {
    fn from(state: readings::BatteryState) -> Self {
        match state {
            readings::BatteryState::Charging => Self::Charging,
            readings::BatteryState::Discharging => Self::Discharging,
            readings::BatteryState::Full => Self::Full,
            readings::BatteryState::NotCharging => Self::NotCharging,
            readings::BatteryState::Unknown => Self::Unknown,
        }
    }
}

impl From<BatteryState> for readings::BatteryState {
    fn from(state: BatteryState) -> Self {
        match state {
            BatteryState::Charging => Self::Charging,
            BatteryState::Discharging => Self::Discharging,
            BatteryState::Full => Self::Full,
            BatteryState::NotCharging => Self::NotCharging,
            BatteryState::Unknown => Self::Unknown,
        }
    }
}

impl From<&Battery> for BatchBattery {
    fn from(battery: &Battery) -> Self {
        Self {
            name: battery.name.clone(),
            state: battery.state.into(),
            seconds_left: battery.seconds_left,
            health_percent: battery.health_percent,
        }
    }
}

/// At most [`MAX_BATTERIES`] batteries with unique names, each a label, a time left of at most
/// `i32::MAX` and a health of at most 100 (K8 Section 4).
pub(super) fn batteries_fit(batteries: Option<&[BatchBattery]>) -> bool {
    let batteries = batteries.unwrap_or_default();
    let names: BTreeSet<&str> = batteries.iter().map(|one| one.name.as_str()).collect();
    names.len() == batteries.len()
        && batteries.len() <= MAX_BATTERIES
        && batteries.iter().all(|one| {
            is_label(&one.name)
                && one.seconds_left.is_none_or(|left| left <= MAX_SECONDS_LEFT)
                && one.health_percent.is_none_or(|health| health <= 100)
        })
}
