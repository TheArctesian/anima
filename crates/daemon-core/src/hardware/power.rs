//! Where the power comes from (CAP-004, AWK-006): the parse of `pmset -g batt` on macOS and of
//! the entries of `/sys/class/power_supply` on Linux. The desktop's stay-awake control and the
//! daemon read with these same rules.

use super::PowerSource;

/// What the power source reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    /// Mains power, or a computer with no battery.
    Mains,
    /// The battery, at this percentage.
    Battery { percent: u8 },
}

impl From<Power> for PowerSource {
    fn from(power: Power) -> Self {
        match power {
            Power::Mains => Self::Mains,
            Power::Battery { .. } => Self::Battery,
        }
    }
}

/// The power source `pmset -g batt` reports. Its first line names the source in quotes
/// (`'Battery Power'`, `'AC Power'`); a battery line holds the charge as `NN%`.
#[must_use]
pub fn from_pmset(report: &str) -> Power {
    let on_battery = report
        .lines()
        .next()
        .is_some_and(|line| line.contains("'Battery Power'"));
    match (on_battery, percent_in(report)) {
        (true, Some(percent)) => Power::Battery { percent },
        _ => Power::Mains,
    }
}

/// The number before the first `%`.
fn percent_in(report: &str) -> Option<u8> {
    let before = &report[..report.find('%')?];
    let digits = before
        .rfind(|character: char| !character.is_ascii_digit())
        .map_or(before, |last| &before[last + 1..]);
    digits.parse().ok().filter(|percent| *percent <= 100)
}

/// One entry of `/sys/class/power_supply`, as its files read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Supply {
    /// `type`: `Battery`, `Mains`, `USB`, …
    pub kind: String,
    /// `scope`: `Device` for the battery of a mouse or a keyboard.
    pub scope: String,
    /// `status`: `Discharging`, `Charging`, `Full`, …
    pub status: String,
    /// `capacity`, in percent.
    pub capacity: Option<u8>,
}

/// The power source the Linux power supplies report: the battery when a system battery
/// discharges. Several batteries give the lowest charge.
#[must_use]
pub fn from_supplies(supplies: &[Supply]) -> Power {
    supplies
        .iter()
        .filter(|supply| supply.kind == "Battery" && supply.scope != "Device")
        .filter(|supply| supply.status == "Discharging")
        .filter_map(|supply| supply.capacity)
        .min()
        .map_or(Power::Mains, |percent| Power::Battery { percent })
}
