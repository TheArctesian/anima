//! The battery readers (MET-032): the files of a Linux power supply and `pmset -g batt`, on the
//! forms laptops write and on hostile text. The `pmset` texts follow the lines macOS 14 and 15
//! print; the sysfs files are those of the kernel's power supply class.

use crate::readings::batteries::{MAX_BATTERIES, batteries, linux_battery, pmset};
use crate::readings::{Battery, BatteryState};

/// A laptop battery while it discharges, with energy counters.
const BAT0: [(&str, &str); 9] = [
    ("type", "Battery\n"),
    ("scope", ""),
    ("present", "1\n"),
    ("capacity", "85\n"),
    ("status", "Discharging\n"),
    ("time_to_empty_now", "9000\n"),
    ("time_to_full_now", "0\n"),
    ("energy_full", "45600000\n"),
    ("energy_full_design", "50000000\n"),
];

fn battery(name: &str, percent: Option<f64>, state: BatteryState) -> Battery {
    Battery {
        name: name.to_owned(),
        percent,
        state,
        seconds_left: None,
        health_percent: None,
    }
}

/// `BAT0` with the file `file` holding `text` in place of what it held.
fn with(file: &'static str, text: &'static str) -> Vec<(&'static str, &'static str)> {
    let mut files: Vec<(&str, &str)> = BAT0.iter().copied().filter(|(f, _)| *f != file).collect();
    files.push((file, text));
    files
}

#[test]
fn met_032_a_discharging_linux_battery_gives_charge_time_to_empty_and_health() {
    let read = linux_battery("BAT0", &BAT0).expect("a battery");
    assert_eq!(
        read,
        Battery {
            seconds_left: Some(9000),
            health_percent: Some(91),
            ..battery("BAT0", Some(85.0), BatteryState::Discharging)
        }
    );
}

#[test]
fn met_032_each_linux_status_maps_to_its_state_and_charging_takes_the_time_to_full() {
    for (status, state) in [
        ("Charging", BatteryState::Charging),
        ("Discharging", BatteryState::Discharging),
        ("Full", BatteryState::Full),
        ("Not charging", BatteryState::NotCharging),
        ("Unknown", BatteryState::Unknown),
        ("Sparkling", BatteryState::Unknown),
    ] {
        let files = with("status", status);
        assert_eq!(
            linux_battery("BAT0", &files).unwrap().state,
            state,
            "{status}"
        );
    }
    let mut charging = with("time_to_full_now", "1800");
    charging.retain(|(file, _)| *file != "status");
    charging.push(("status", "Charging"));
    assert_eq!(
        linux_battery("BAT0", &charging).unwrap().seconds_left,
        Some(1800)
    );
}

#[test]
fn met_032_a_mouse_a_mains_supply_and_an_absent_battery_are_not_batteries() {
    let mut mouse = BAT0.to_vec();
    mouse[1] = ("scope", "Device\n");
    assert_eq!(linux_battery("hidpp_battery_0", &mouse), None);
    let mut mains = BAT0.to_vec();
    mains[0] = ("type", "Mains\n");
    assert_eq!(linux_battery("AC", &mains), None);
    let mut absent = BAT0.to_vec();
    absent[2] = ("present", "0\n");
    assert_eq!(linux_battery("BAT1", &absent), None);
    assert_eq!(linux_battery("BAT0", &[]), None, "no type file");
    assert_eq!(linux_battery("", &BAT0), None, "a name that is no label");
}

#[test]
fn met_032_a_charge_outside_0_to_100_and_counters_that_do_not_read_are_left_out() {
    for text in ["101", "-1", "NaN", "full", ""] {
        let files = with("capacity", text);
        assert_eq!(
            linux_battery("BAT0", &files).unwrap().percent,
            None,
            "{text}"
        );
    }
    let no_estimate = with("time_to_empty_now", "0");
    assert_eq!(
        linux_battery("BAT0", &no_estimate).unwrap().seconds_left,
        None
    );
    let huge = with("time_to_empty_now", "4294967295");
    assert_eq!(linux_battery("BAT0", &huge).unwrap().seconds_left, None);
    let no_design = with("energy_full_design", "0");
    assert_eq!(
        linux_battery("BAT0", &no_design).unwrap().health_percent,
        None
    );
}

#[test]
fn met_032_health_falls_back_to_charge_counters_and_is_held_to_100() {
    let mut charge: Vec<(&str, &str)> = BAT0[..6].to_vec();
    charge.extend([
        ("charge_full", "5200000"),
        ("charge_full_design", "5000000"),
    ]);
    assert_eq!(
        linux_battery("BAT0", &charge).unwrap().health_percent,
        Some(100)
    );
}

#[test]
fn met_032_batteries_are_sorted_kept_once_and_at_most_eight() {
    let many = (0..10)
        .rev()
        .map(|i| battery(&format!("BAT{i}"), None, BatteryState::Full));
    let kept = batteries(many.chain([battery("BAT0", Some(1.0), BatteryState::Full)]));
    assert_eq!(kept.len(), MAX_BATTERIES);
    assert_eq!(kept[0].name, "BAT0");
    assert_eq!(kept[7].name, "BAT7");
}

#[test]
fn met_032_pmset_gives_the_internal_battery_its_state_and_time() {
    let discharging = b"Now drawing from 'Battery Power'\n \
-InternalBattery-0 (id=1234567)\t85%; discharging; 4:12 remaining present: true\n";
    assert_eq!(
        pmset(discharging),
        Some(vec![Battery {
            seconds_left: Some(15_120),
            ..battery("InternalBattery-0", Some(85.0), BatteryState::Discharging)
        }])
    );
    for (line, state, seconds) in [
        (
            "56%; charging; 1:23 remaining",
            BatteryState::Charging,
            Some(4980),
        ),
        (
            "99%; finishing charge; (no estimate)",
            BatteryState::Charging,
            None,
        ),
        ("100%; charged; 0:00 remaining", BatteryState::Full, None),
        (
            "80%; AC attached; not charging",
            BatteryState::NotCharging,
            None,
        ),
        (
            "40%; discharging; (no estimate)",
            BatteryState::Discharging,
            None,
        ),
        (
            "40%; draining fast; 1:00 remaining",
            BatteryState::Unknown,
            None,
        ),
    ] {
        let text = format!(
            "Now drawing from 'AC Power'\n -InternalBattery-0 (id=99)\t{line} present: true\n"
        );
        let read = pmset(text.as_bytes()).unwrap();
        assert_eq!(
            (read[0].state, read[0].seconds_left),
            (state, seconds),
            "{line}"
        );
    }
}

#[test]
fn met_032_pmset_of_a_desktop_a_ups_or_another_command_gives_no_battery() {
    assert_eq!(pmset(b"Now drawing from 'AC Power'\n"), Some(Vec::new()));
    let ups = b"Now drawing from 'AC Power'\n \
-CP1500 UPS (id=1)\t100%; AC attached; not charging present: true\n";
    assert_eq!(pmset(ups), Some(Vec::new()), "a UPS powers it from outside");
    let absent = b"Now drawing from 'AC Power'\n \
-InternalBattery-0 (id=1)\t0%; AC attached; not charging present: false\n";
    assert_eq!(pmset(absent), Some(Vec::new()));
    assert_eq!(pmset(b"pmset: command not found\n"), None);
    assert_eq!(pmset(b""), None);
    let hostile = b"Now drawing from 'Battery Power'\n \
-InternalBattery-0 (id=1)\t250%; discharging; 99999999:00 remaining present: true\n";
    let read = pmset(hostile).unwrap();
    assert_eq!((read[0].percent, read[0].seconds_left), (None, None));
}
