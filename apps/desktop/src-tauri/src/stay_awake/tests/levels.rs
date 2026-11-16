use crate::stay_awake::{
    level::{Held, Level, Pause, Power, Wanted, held},
    saved,
};

fn wanted(level: Level, battery_floor: Option<u8>) -> Wanted {
    Wanted {
        level,
        battery_floor,
    }
}

const LOW: Held = Held {
    level: Level::Off,
    pause: Some(Pause::LowBattery),
};

#[test]
fn off_holds_nothing_whatever_the_power_or_the_lid() {
    let off = wanted(Level::Off, Some(50));
    assert_eq!(
        held(off, Power::Battery { percent: 5 }, true, true),
        Held::default()
    );
    assert_eq!(held(off, Power::Mains, false, false), Held::default());
}

#[test]
fn a_chosen_level_is_held_on_mains_power_and_on_a_battery_with_no_floor() {
    for level in [Level::Awake, Level::LidClosed] {
        let whole = Held { level, pause: None };
        assert_eq!(
            held(wanted(level, Some(50)), Power::Mains, true, false),
            whole
        );
        let on_battery = Power::Battery { percent: 1 };
        assert_eq!(held(wanted(level, None), on_battery, true, false), whole);
    }
}

#[test]
fn on_battery_below_the_floor_the_control_is_held_off() {
    let choice = wanted(Level::Awake, Some(20));
    assert_eq!(
        held(choice, Power::Battery { percent: 19 }, true, false),
        LOW
    );
    let at_the_floor = held(choice, Power::Battery { percent: 20 }, true, false);
    assert_eq!(at_the_floor.level, Level::Awake);
}

#[test]
fn a_low_battery_pause_lasts_until_mains_power_returns() {
    let choice = wanted(Level::LidClosed, Some(20));
    assert_eq!(
        held(choice, Power::Battery { percent: 22 }, true, true),
        LOW
    );
    let back = held(choice, Power::Mains, true, true);
    assert_eq!(
        back,
        Held {
            level: Level::LidClosed,
            pause: None
        }
    );
}

#[test]
fn the_lid_level_falls_to_keep_awake_when_the_lid_is_not_ready() {
    let fallen = held(wanted(Level::LidClosed, None), Power::Mains, false, false);
    assert_eq!(
        fallen,
        Held {
            level: Level::Awake,
            pause: Some(Pause::LidNotReady)
        }
    );
}

#[test]
fn a_floor_outside_the_range_moves_to_the_nearest_end() {
    let choice = Wanted::default();
    assert_eq!(choice.with_floor(Some(0)).battery_floor, Some(5));
    assert_eq!(choice.with_floor(Some(100)).battery_floor, Some(95));
    assert_eq!(choice.with_floor(Some(30)).battery_floor, Some(30));
    assert_eq!(choice.with_floor(None).battery_floor, None);
}

#[test]
fn the_choice_is_read_back_as_it_was_saved() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("made-on-save");
    let choice = wanted(Level::LidClosed, Some(30));
    saved::save(&folder, choice).unwrap();
    assert_eq!(saved::load(&folder), choice);
}

#[test]
fn a_missing_or_damaged_file_is_the_default_and_a_wild_floor_is_brought_in_range() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(saved::load(dir.path()), Wanted::default());
    let file = dir.path().join("stay-awake.json");
    std::fs::write(&file, "not json").unwrap();
    assert_eq!(saved::load(dir.path()), Wanted::default());
    std::fs::write(&file, r#"{"level":"awake","batteryFloor":200}"#).unwrap();
    assert_eq!(saved::load(dir.path()), wanted(Level::Awake, Some(95)));
}
