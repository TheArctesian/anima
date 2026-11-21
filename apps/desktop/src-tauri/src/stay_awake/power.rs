//! Where this computer's power comes from now (AWK-006): on macOS from `pmset -g batt`, on
//! Linux from `/sys/class/power_supply`. Neither needs more rights. A state that cannot be
//! read is `Mains`, so a read failure never turns the control off. The parse is the daemon's
//! (the crate `power`), so the two read the power source alike (CAP-004).

use std::{ffi::OsStr, fs, path::Path, time::Duration};

use super::{
    level::Power,
    platform::{Os, Platform},
};
use crate::os_program::{self, AfterLimit, Limit, Stdout};

const POWER_SUPPLIES: &str = "/sys/class/power_supply";

/// `pmset -g` prints a few lines at once.
const PMSET_LIMIT: Limit = Limit {
    patience: Duration::from_secs(3),
    after: AfterLimit::Kill,
};

const PMSET_OUTPUT: Stdout = Stdout::Capture { max_bytes: 4096 };

pub use ::power::{Supply, from_pmset, from_supplies};

fn supply_at(entry: &Path) -> Supply {
    let field = |name: &str| {
        fs::read_to_string(entry.join(name)).map_or_default(|text| text.trim().to_owned())
    };
    Supply {
        kind: field("type"),
        scope: field("scope"),
        status: field("status"),
        capacity: field("capacity")
            .parse()
            .ok()
            .filter(|percent| *percent <= 100),
    }
}

/// The supplies under `root` (`/sys/class/power_supply`); none when it cannot be read.
fn supplies_under(root: &Path) -> Vec<Supply> {
    fs::read_dir(root).map_or_default(|entries| {
        entries
            .flatten()
            .map(|entry| supply_at(&entry.path()))
            .collect()
    })
}

/// The output of `pmset` with `args` as text, or `None` when it did not run.
pub fn pmset_report(pmset: &str, args: &[&str]) -> Option<String> {
    let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    os_program::run(pmset, &args, PMSET_LIMIT, PMSET_OUTPUT)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// This computer's power source now. Blocks for a short program run on macOS.
#[must_use]
pub fn read(platform: &Platform) -> Power {
    match platform.os {
        Os::Mac => pmset_report(&platform.pmset, &["-g", "batt"])
            .map_or(Power::Mains, |report| from_pmset(&report)),
        Os::Linux => from_supplies(&supplies_under(Path::new(POWER_SUPPLIES))),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Power, Supply, from_pmset, from_supplies, supplies_under};

    const ON_MAINS: &str = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t100%; charged; 0:00 remaining present: true\n";
    const ON_BATTERY: &str = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t37%; discharging; 2:10 remaining present: true\n";

    fn battery(status: &str, capacity: u8) -> Supply {
        Supply {
            kind: "Battery".into(),
            status: status.into(),
            capacity: Some(capacity),
            ..Supply::default()
        }
    }

    #[test]
    fn a_mac_on_its_battery_reports_the_charge() {
        assert_eq!(from_pmset(ON_BATTERY), Power::Battery { percent: 37 });
    }

    #[test]
    fn a_mac_on_mains_power_is_on_mains_whatever_the_charge() {
        assert_eq!(from_pmset(ON_MAINS), Power::Mains);
    }

    #[test]
    fn a_mac_with_no_battery_or_an_unreadable_report_is_on_mains() {
        assert_eq!(from_pmset("Now drawing from 'AC Power'\n"), Power::Mains);
        assert_eq!(
            from_pmset("Now drawing from 'Battery Power'\n"),
            Power::Mains
        );
        assert_eq!(from_pmset(""), Power::Mains);
        assert_eq!(
            from_pmset("Now drawing from 'Battery Power'\n -x\t999%; discharging\n"),
            Power::Mains
        );
    }

    #[test]
    fn a_discharging_linux_battery_reports_the_lowest_charge() {
        let supplies = [battery("Discharging", 64), battery("Discharging", 41)];
        assert_eq!(from_supplies(&supplies), Power::Battery { percent: 41 });
    }

    #[test]
    fn a_charging_or_full_linux_battery_is_mains_power() {
        assert_eq!(from_supplies(&[battery("Charging", 12)]), Power::Mains);
        assert_eq!(from_supplies(&[battery("Full", 100)]), Power::Mains);
        assert_eq!(from_supplies(&[]), Power::Mains);
    }

    #[test]
    fn the_battery_of_a_mouse_is_not_the_computers_battery() {
        let mouse = Supply {
            scope: "Device".into(),
            ..battery("Discharging", 5)
        };
        assert_eq!(from_supplies(&[mouse]), Power::Mains);
    }

    #[test]
    fn the_supplies_are_read_from_the_files_of_each_entry() {
        let root = tempfile::tempdir().unwrap();
        let entry = root.path().join("BAT0");
        fs::create_dir(&entry).unwrap();
        fs::write(entry.join("type"), "Battery\n").unwrap();
        fs::write(entry.join("status"), "Discharging\n").unwrap();
        fs::write(entry.join("capacity"), "58\n").unwrap();
        assert_eq!(
            from_supplies(&supplies_under(root.path())),
            Power::Battery { percent: 58 }
        );
        assert_eq!(supplies_under(&root.path().join("missing")), Vec::new());
    }
}
