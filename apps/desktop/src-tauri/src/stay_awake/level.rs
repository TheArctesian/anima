//! The level the computer is held at, from the operator's choice, the power source and the
//! lid set-up (AWK-001, AWK-006). Pure: it reads no clock, no file and no OS state.

use serde::{Deserialize, Serialize};

/// The three states of the control (AWK-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    /// The computer sleeps as the OS decides.
    #[default]
    Off,
    /// No idle sleep; the display can sleep (AWK-002).
    Awake,
    /// As `Awake`, and the computer stays on with the lid closed (AWK-003).
    LidClosed,
}

/// The lowest and highest battery floor the operator can set, in percent.
pub const FLOOR_RANGE: std::ops::RangeInclusive<u8> = 5..=95;

/// What the operator chose. Saved between runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wanted {
    pub level: Level,
    /// On battery below this percentage the control is off (AWK-006). `None`: no floor.
    pub battery_floor: Option<u8>,
}

impl Wanted {
    /// The same choice with a floor the control accepts: one outside `FLOOR_RANGE` is moved to
    /// the nearest end.
    #[must_use]
    pub fn with_floor(self, floor: Option<u8>) -> Self {
        let battery_floor =
            floor.map(|percent| percent.clamp(*FLOOR_RANGE.start(), *FLOOR_RANGE.end()));
        Self {
            battery_floor,
            ..self
        }
    }
}

/// Where the computer's power comes from now: one reader with the daemon (CAP-004).
pub use ::power::Power;

/// Why the computer is held at a lower level than the operator chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pause {
    /// On battery, below the operator's floor (AWK-006). It ends on mains power.
    LowBattery,
    /// "Keep awake with the lid closed" is chosen, and this computer cannot do it now: the
    /// set-up step is not done, or the OS refused.
    LidNotReady,
    /// The OS refused every hold. `held` never gives it: the control does, after it tried.
    Unavailable,
}

/// The level the computer is held at now, and why it is lower than the choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Held {
    pub level: Level,
    pub pause: Option<Pause>,
}

/// The level to hold for `wanted` on `power`.
///
/// `lid_ready`: this computer can keep awake with the lid closed now. `was_low`: the last
/// answer was a `LowBattery` pause; such a pause lasts until mains power returns, so a reading
/// that moves around the floor does not turn the control on and off.
#[must_use]
pub fn held(wanted: Wanted, power: Power, lid_ready: bool, was_low: bool) -> Held {
    if wanted.level == Level::Off {
        return Held::default();
    }
    if low(wanted.battery_floor, power, was_low) {
        return Held {
            level: Level::Off,
            pause: Some(Pause::LowBattery),
        };
    }
    if wanted.level == Level::LidClosed && !lid_ready {
        return Held {
            level: Level::Awake,
            pause: Some(Pause::LidNotReady),
        };
    }
    Held {
        level: wanted.level,
        pause: None,
    }
}

fn low(floor: Option<u8>, power: Power, was_low: bool) -> bool {
    match (floor, power) {
        (Some(floor), Power::Battery { percent }) => was_low || percent < floor,
        _ => false,
    }
}
