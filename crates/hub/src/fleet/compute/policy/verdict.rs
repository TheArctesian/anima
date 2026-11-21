//! Whether a node takes model work now (CMP-002, CAP-010 to CAP-012). Pure: the caller gives
//! the policy, the switch, the last hardware values and the time.

use chrono::{DateTime, Utc};
use daemon_core::hardware::{Hardware, PowerSource};

use super::rules::DevicePolicy;

/// The one switch "takes work" of a node (USE-004, OD-C8): at the machine and in the hub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkSwitch {
    /// The machine never asked this hub for work, so the hub does not know its switch.
    Unknown,
    /// The switch at the machine is off.
    OffAtMachine,
    /// The operator set the machine off in the hub, or the hub holds it off for an hour.
    OffInHub,
    On,
}

/// Why a node takes no model work now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    NoSwitch,
    SwitchOff,
    OffInHub,
    QuietHours,
    OnBattery,
    OperatorActive,
}

/// What the node reported last that the policy reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeState {
    pub power: Option<PowerSource>,
    pub operator_active: Option<bool>,
}

impl From<&Hardware> for NodeState {
    fn from(hardware: &Hardware) -> Self {
        Self {
            power: hardware.power_source,
            operator_active: hardware.operator_active,
        }
    }
}

/// `Ok` when the node takes model work at `now`, else the first reason it does not. The switch
/// comes first. A value the node did not report does not stop it: a computer with no battery
/// reports mains, and an older daemon reports nothing.
///
/// # Errors
/// [`Stop`] is the reason, not a failure.
pub fn takes_model_work(
    policy: &DevicePolicy,
    switch: WorkSwitch,
    node: NodeState,
    now: DateTime<Utc>,
) -> Result<(), Stop> {
    match switch {
        WorkSwitch::Unknown => return Err(Stop::NoSwitch),
        WorkSwitch::OffAtMachine => return Err(Stop::SwitchOff),
        WorkSwitch::OffInHub => return Err(Stop::OffInHub),
        WorkSwitch::On => {}
    }
    if policy.quiet_hours.is_some_and(|hours| hours.contain(now)) {
        return Err(Stop::QuietHours);
    }
    if policy.mains_only && node.power == Some(PowerSource::Battery) {
        return Err(Stop::OnBattery);
    }
    if policy.idle_only && node.operator_active == Some(true) {
        return Err(Stop::OperatorActive);
    }
    Ok(())
}
