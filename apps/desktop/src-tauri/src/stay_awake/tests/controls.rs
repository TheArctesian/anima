use std::fs;

use super::rig::{Rig, eventually, running};
use crate::stay_awake::{
    control::{Control, ControlError, Status},
    error::StayAwakeError,
    level::{Level, Pause, Power},
    lid_set_up::{Lid, SetUpError},
    platform::Os,
};

fn control(rig: &Rig) -> Control {
    Control::open(rig.platform.clone(), rig.folder())
}

#[test]
fn a_new_control_is_off_and_a_choice_is_kept_for_the_next_run() {
    let rig = Rig::new(Os::Linux);
    let mut first = control(&rig);
    assert_eq!(first.refresh().level, Level::Off);
    first.set_floor(Some(30)).unwrap();
    let chosen = first.choose(Level::Awake).unwrap();
    assert_eq!(
        (chosen.level, chosen.held, chosen.pause),
        (Level::Awake, Level::Awake, None)
    );
    drop(first);
    let again = control(&rig).refresh();
    assert_eq!((again.level, again.battery_floor), (Level::Awake, Some(30)));
}

#[test]
fn the_hold_program_runs_while_a_level_is_held_and_ends_at_release() {
    let mut rig = Rig::new(Os::Linux);
    let mark = format!("anima-control-test-{}", std::process::id());
    rig.platform.inhibit = rig.stand_in("inhibit", &format!("sh -c 'sleep 30; : {mark}' & wait"));
    let mut control = control(&rig);
    control.choose(Level::Awake).unwrap();
    assert!(eventually(|| running(&mark)));
    control.release();
    assert!(eventually(|| !running(&mark)));
    assert_eq!(
        control.refresh().level,
        Level::Awake,
        "the choice stays saved"
    );
}

#[test]
fn the_lid_level_without_the_set_up_step_holds_keep_awake_and_says_why() {
    let rig = Rig::new(Os::Mac);
    let status = control(&rig).choose(Level::LidClosed).unwrap();
    assert_eq!(
        status,
        Status {
            level: Level::LidClosed,
            battery_floor: None,
            held: Level::Awake,
            pause: Some(Pause::LidNotReady),
            lid: Lid::NeedsSetUp,
        }
    );
}

#[test]
fn the_lid_level_that_the_os_refuses_holds_keep_awake() {
    let mut rig = Rig::mac_set_up();
    rig.platform.sudo = rig.stand_in("sudo", "exit 1");
    let status = control(&rig).choose(Level::LidClosed).unwrap();
    assert_eq!(
        (status.held, status.pause),
        (Level::Awake, Some(Pause::LidNotReady))
    );
}

#[test]
fn a_computer_that_refuses_every_hold_is_held_off_and_says_so() {
    let mut rig = Rig::new(Os::Linux);
    rig.platform.inhibit = "/nonexistent/anima-inhibit".into();
    let status = control(&rig).choose(Level::Awake).unwrap();
    assert_eq!(
        (status.held, status.pause),
        (Level::Off, Some(Pause::Unavailable))
    );
}

#[test]
fn below_the_floor_on_battery_the_holds_end_until_mains_power_returns() {
    let rig = Rig::mac_set_up();
    let mut control = control(&rig);
    control.set_floor(Some(20)).unwrap();
    control.choose(Level::LidClosed).unwrap();
    let low = control.apply(Power::Battery { percent: 12 });
    assert_eq!((low.held, low.pause), (Level::Off, Some(Pause::LowBattery)));
    assert_eq!(
        rig.sudo_calls().last().unwrap(),
        "sudo -n pmset -a disablesleep 0"
    );
    assert!(!rig.marker().exists());
    let still = control.apply(Power::Battery { percent: 25 });
    assert_eq!(still.held, Level::Off);
    let back = control.apply(Power::Mains);
    assert_eq!((back.held, back.pause), (Level::LidClosed, None));
    assert_eq!(
        rig.sudo_calls().last().unwrap(),
        "sudo -n pmset -a disablesleep 1"
    );
}

#[test]
fn a_start_after_a_failed_run_sets_the_sleep_setting_back() {
    let rig = Rig::mac_set_up();
    fs::write(rig.marker(), "").unwrap();
    let _control = control(&rig);
    assert_eq!(rig.sudo_calls(), ["sudo -n pmset -a disablesleep 0"]);
    assert!(!rig.marker().exists());
}

#[test]
fn a_choice_that_cannot_be_saved_changes_nothing() {
    let rig = Rig::new(Os::Linux);
    let mut control = control(&rig);
    // A file where the folder must be: the save fails.
    fs::write(rig.folder(), "").unwrap();
    assert_eq!(control.choose(Level::Awake), Err(ControlError::Unsaved));
    assert_eq!(control.refresh().level, Level::Off);
}

#[test]
fn the_state_and_each_error_cross_to_the_client_in_a_fixed_shape() {
    let status = Status {
        level: Level::LidClosed,
        battery_floor: Some(20),
        held: Level::Awake,
        pause: Some(Pause::LidNotReady),
        lid: Lid::NeedsSetUp,
    };
    assert_eq!(
        serde_json::to_value(status).unwrap(),
        serde_json::json!({
            "level": "lid-closed", "batteryFloor": 20, "held": "awake",
            "pause": "lid-not-ready", "lid": "needs-set-up"
        })
    );
    let codes = [
        (StayAwakeError::Caller, "caller"),
        (ControlError::Unsaved.into(), "unsaved"),
        (
            ControlError::SetUp(SetUpError::Unsupported).into(),
            "unsupported",
        ),
        (ControlError::SetUp(SetUpError::Refused).into(), "refused"),
        (StayAwakeError::Failed, "failed"),
    ];
    for (error, code) in codes {
        let json = serde_json::to_value(error).unwrap();
        assert_eq!(json["code"], code);
        assert_eq!(json["message"], error.to_string());
    }
}

#[test]
fn release_on_a_mac_sets_the_sleep_setting_back() {
    let rig = Rig::mac_set_up();
    let mut control = control(&rig);
    control.choose(Level::LidClosed).unwrap();
    assert!(rig.marker().exists());
    control.release();
    assert_eq!(
        rig.sudo_calls().last().unwrap(),
        "sudo -n pmset -a disablesleep 0"
    );
    assert!(!rig.marker().exists());
}

#[test]
fn a_computer_that_loses_the_idle_hold_also_ends_the_lid_hold() {
    let mut rig = Rig::mac_set_up();
    rig.platform.caffeinate = "/nonexistent/anima-caffeinate".into();
    let status = control(&rig).choose(Level::LidClosed).unwrap();
    assert_eq!(
        (status.held, status.pause),
        (Level::Off, Some(Pause::Unavailable))
    );
    assert_eq!(
        rig.sudo_calls().last().unwrap(),
        "sudo -n pmset -a disablesleep 0"
    );
    assert!(!rig.marker().exists());
}

/// A `pmset` that reports the battery at `percent` and counts its runs in `runs`.
fn pmset_on_battery(rig: &Rig, percent: u8) -> std::path::PathBuf {
    let runs = rig.dir.path().join("pmset-runs");
    let body = format!(
        "echo x >> '{}'; echo \"Now drawing from 'Battery Power'\"; echo ' -InternalBattery-0 {percent}%; discharging'",
        runs.display()
    );
    rig.stand_in("pmset", &body);
    runs
}

#[test]
fn the_operators_own_change_ends_a_low_battery_pause() {
    let rig = Rig::new(Os::Mac);
    pmset_on_battery(&rig, 25);
    let mut control = control(&rig);
    control.set_floor(Some(20)).unwrap();
    control.choose(Level::Awake).unwrap();
    let low = control.apply(Power::Battery { percent: 12 });
    assert_eq!(low.pause, Some(Pause::LowBattery));
    assert_eq!(
        control.refresh().pause,
        Some(Pause::LowBattery),
        "25% is still paused"
    );
    let chosen = control.choose(Level::Awake).unwrap();
    assert_eq!((chosen.held, chosen.pause), (Level::Awake, None));
}

#[test]
fn the_power_source_is_read_only_when_a_floor_can_change_the_answer() {
    let rig = Rig::new(Os::Mac);
    let runs = pmset_on_battery(&rig, 80);
    let mut control = control(&rig);
    control.choose(Level::Awake).unwrap();
    control.refresh();
    assert!(!runs.exists());
    control.set_floor(Some(20)).unwrap();
    assert!(runs.exists());
}

#[test]
fn a_borrowed_sleep_setting_is_looked_at_again_and_taken_when_the_other_program_ends_it() {
    let rig = Rig::mac_set_up();
    rig.stand_in("pmset", "echo ' SleepDisabled 1'");
    let mut control = control(&rig);
    let borrowed = control.choose(Level::LidClosed).unwrap();
    assert_eq!(borrowed.held, Level::LidClosed);
    assert_eq!(rig.sudo_calls(), Vec::<String>::new());
    rig.stand_in("pmset", "echo ' SleepDisabled 0'");
    assert_eq!(control.refresh().held, Level::LidClosed);
    assert_eq!(rig.sudo_calls(), ["sudo -n pmset -a disablesleep 1"]);
}
