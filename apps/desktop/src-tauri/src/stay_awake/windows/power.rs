//! Windows battery readings feed the shared floor and hysteresis rules.
use super::{level::Power, platform::Platform};
pub fn read(_: &Platform) -> Power {
    anima_windows::power::battery_percent()
        .map_or(Power::Mains, |percent| Power::Battery { percent })
}
