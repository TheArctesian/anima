//! The two administrator steps of the lid-closed level, with a stand-in for the dialog.

use std::{
    fs,
    time::{Duration, Instant},
};

use super::rig::Rig;
use crate::stay_awake::{
    control::{Control, ControlError, Status, Step},
    level::{Level, Pause},
    lid_set_up::{Lid, SetUpError},
    platform::Os,
};

fn control(rig: &Rig) -> Control {
    Control::open(rig.platform.clone(), rig.folder())
}

/// One administrator step from start to end, as the command runs it.
fn step(control: &mut Control, step: Step, now: Instant) -> Result<Status, ControlError> {
    let Some(ask) = control.begin(step, now)? else {
        return Ok(control.refresh());
    };
    let asked = ask.run();
    control.finish(&ask, asked, now)
}

#[test]
fn removing_the_set_up_sets_sleep_back_before_the_rule_is_deleted() {
    let rig = Rig::mac_set_up();
    let mut control = control(&rig);
    control.choose(Level::LidClosed).unwrap();
    step(&mut control, Step::Remove, Instant::now()).unwrap();
    let calls = rig.calls();
    let set_back = calls
        .iter()
        .position(|call| call.ends_with("disablesleep 0"));
    let removed = calls.iter().position(|call| call.starts_with("osascript "));
    assert!(set_back.is_some() && set_back < removed, "{calls:?}");
    assert!(!calls[removed.unwrap()].contains("pmset"), "{calls:?}");
}

#[test]
fn removing_the_set_up_has_root_set_sleep_back_when_anima_could_not() {
    let mut rig = Rig::mac_set_up();
    // An earlier run changed the setting; now the rule refuses, so the set-back at the start fails.
    fs::write(rig.marker(), "").unwrap();
    rig.platform.sudo = rig.stand_in("sudo", "exit 1");
    let mut control = control(&rig);
    assert!(rig.marker().exists());
    step(&mut control, Step::Remove, Instant::now()).unwrap();
    let asked = rig.calls().pop().unwrap();
    assert!(
        asked.contains("/usr/bin/pmset -a disablesleep 0; /bin/rm -f"),
        "{asked}"
    );
    assert!(!rig.marker().exists());
}

#[test]
fn a_step_that_would_change_nothing_shows_no_dialog() {
    let rig = Rig::mac_set_up();
    let mut ready = control(&rig);
    let status = step(&mut ready, Step::SetUp, Instant::now()).unwrap();
    assert_eq!(status.lid, Lid::Ready);
    let bare = Rig::new(Os::Mac);
    let mut not_set_up = control(&bare);
    step(&mut not_set_up, Step::Remove, Instant::now()).unwrap();
    assert_eq!((rig.calls(), bare.calls()), (Vec::new(), Vec::new()));
}

#[test]
fn after_a_refused_step_the_next_one_shows_no_dialog_for_a_short_time() {
    let mut rig = Rig::new(Os::Mac);
    let seen = rig.dir.path().join("dialogs");
    let body = format!("echo x >> '{}'; exit 1", seen.display());
    rig.platform.osascript = rig.stand_in("osascript", &body);
    let mut control = control(&rig);
    let start = Instant::now();
    let dialogs = || fs::read_to_string(&seen).unwrap().lines().count();
    let refused = Err(ControlError::SetUp(SetUpError::Refused));
    assert_eq!(step(&mut control, Step::SetUp, start), refused);
    let soon = start + Duration::from_secs(5);
    assert_eq!(step(&mut control, Step::SetUp, soon), refused);
    assert_eq!(dialogs(), 1);
    let later = start + Duration::from_secs(20);
    assert_eq!(step(&mut control, Step::SetUp, later), refused);
    assert_eq!(dialogs(), 2);
}

#[test]
fn while_a_dialog_is_open_no_second_one_opens_and_no_lid_hold_is_taken() {
    let rig = Rig::mac_set_up();
    let mut control = control(&rig);
    control.choose(Level::LidClosed).unwrap();
    let now = Instant::now();
    let refused = ControlError::SetUp(SetUpError::Refused);
    let ask = control.begin(Step::Remove, now).unwrap().expect("a dialog");
    assert_eq!(control.begin(Step::Remove, now).unwrap_err(), refused);
    let during = control.refresh();
    assert_eq!(
        (during.held, during.pause),
        (Level::Awake, Some(Pause::LidNotReady))
    );
    assert_eq!(
        rig.sudo_calls().last().unwrap(),
        "sudo -n pmset -a disablesleep 0"
    );
    // The dialog was cancelled: the rule is still there, and the lid hold comes back.
    let after = control.finish(&ask, Err(SetUpError::Refused), now);
    assert_eq!(after, Err(refused));
    assert_eq!(control.refresh().held, Level::LidClosed);
}

#[test]
fn linux_has_no_administrator_step() {
    let rig = Rig::new(Os::Linux);
    let unsupported = ControlError::SetUp(SetUpError::Unsupported);
    let begun = control(&rig).begin(Step::SetUp, Instant::now());
    assert_eq!(begun.unwrap_err(), unsupported);
}
