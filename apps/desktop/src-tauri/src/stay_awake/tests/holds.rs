use std::fs;

use super::rig::{Rig, eventually, running, short_lived};
#[cfg(target_os = "macos")]
use crate::stay_awake::platform::Platform;
use crate::stay_awake::{
    hold::{self, HoldError, LidHold, sleep_disabled},
    platform::Os,
};

#[test]
fn a_hold_program_that_ends_at_once_was_refused() {
    let mut rig = Rig::new(Os::Linux);
    rig.platform.inhibit = rig.stand_in("inhibit", "exit 1");
    assert_eq!(hold::idle(&rig.platform).unwrap_err(), HoldError::Refused);
}

#[test]
fn a_missing_hold_program_could_not_start() {
    let mut rig = Rig::new(Os::Mac);
    rig.platform.caffeinate = "/nonexistent/anima-caffeinate".into();
    assert_eq!(
        hold::idle(&rig.platform).unwrap_err(),
        HoldError::CouldNotStart
    );
}

#[test]
fn ending_a_hold_ends_its_program_and_what_the_program_started() {
    let mut rig = Rig::new(Os::Linux);
    let mark = format!("anima-hold-test-{}", std::process::id());
    // The stand-in starts a second process, as `systemd-inhibit` starts its command.
    rig.platform.inhibit = rig.stand_in("inhibit", &format!("sh -c 'sleep 30; : {mark}' & wait"));
    let held = hold::idle(&rig.platform).expect("a hold");
    assert!(eventually(|| running(&mark)));
    drop(held);
    assert!(eventually(|| !running(&mark)));
}

#[test]
fn the_linux_inhibitors_block_sleep_and_the_lid_and_follow_the_app() {
    let mut rig = Rig::new(Os::Linux);
    let seen = rig.dir.path().join("seen");
    let body = format!("echo \"$*\" >> '{}'; exec sleep 30", seen.display());
    rig.platform.inhibit = rig.stand_in("inhibit", &body);
    rig.platform.followed = 4242;
    let _idle = hold::idle(&rig.platform).expect("the idle hold");
    let lid = hold::lid(&rig.platform, &rig.marker()).expect("the lid hold");
    assert!(matches!(lid, LidHold::Inhibitor(_)));
    // Each program writes its line when it starts; the two starts are not ordered.
    let seen = fs::read_to_string(seen).unwrap();
    let mut seen: Vec<_> = seen.lines().collect();
    seen.sort_unstable();
    let follows = "--who=Anima --why=Stay awake --mode=block tail --pid=4242 -f /dev/null";
    assert_eq!(
        seen,
        [
            format!("--what=handle-lid-switch {follows}"),
            format!("--what=sleep {follows}")
        ]
    );
}

#[cfg(target_os = "macos")]
#[test]
fn keep_awake_on_a_mac_holds_a_real_assertion_that_ends_with_the_followed_process() {
    let mut followed = short_lived();
    let platform = Platform {
        followed: followed.id(),
        ..Platform::here()
    };
    let mut held = hold::idle(&platform).expect("caffeinate");
    let assertions = std::process::Command::new("/usr/bin/pmset")
        .args(["-g", "assertions"])
        .output()
        .unwrap();
    let assertions = String::from_utf8_lossy(&assertions.stdout).into_owned();
    assert!(assertions.contains("caffeinate"), "{assertions}");
    assert!(held.alive());
    followed.wait().unwrap();
    assert!(eventually(|| !held.alive()));
}

#[test]
fn the_sleep_setting_is_read_from_the_pmset_report() {
    assert!(sleep_disabled(
        "System-wide power settings:\n SleepDisabled\t\t1\nCurrently in use:\n"
    ));
    assert!(!sleep_disabled(" SleepDisabled\t\t0\n sleep 1\n"));
    assert!(!sleep_disabled("Currently in use:\n sleep 1\n"));
}

#[test]
fn a_mac_without_the_set_up_step_takes_no_lid_hold_and_runs_nothing_as_root() {
    let rig = Rig::new(Os::Mac);
    let refused = hold::lid(&rig.platform, &rig.marker()).unwrap_err();
    assert_eq!(refused, HoldError::NotSetUp);
    assert_eq!(rig.calls(), Vec::<String>::new());
    assert!(!rig.marker().exists());
}

#[test]
fn the_lid_hold_on_a_mac_disables_sleep_and_its_end_sets_it_back() {
    let rig = Rig::mac_set_up();
    let marker = rig.marker();
    let held = hold::lid(&rig.platform, &marker).expect("the lid hold");
    assert_eq!(rig.sudo_calls(), ["sudo -n pmset -a disablesleep 1"]);
    assert!(marker.exists());
    hold::release_lid(&rig.platform, &marker, held).unwrap();
    assert_eq!(
        rig.sudo_calls().last().unwrap(),
        "sudo -n pmset -a disablesleep 0"
    );
    assert_eq!(rig.sudo_calls().len(), 2);
    assert!(!marker.exists());
}

#[test]
fn a_sleep_setting_that_a_different_program_made_is_left_alone() {
    let mut rig = Rig::mac_set_up();
    rig.platform.pmset = rig.stand_in("pmset", "echo ' SleepDisabled 1'");
    let marker = rig.marker();
    let held = hold::lid(&rig.platform, &marker).expect("the lid hold");
    assert!(matches!(held, LidHold::Borrowed));
    hold::release_lid(&rig.platform, &marker, held).unwrap();
    assert_eq!(rig.calls(), Vec::<String>::new());
    assert!(!marker.exists());
}

#[test]
fn a_refused_change_of_the_sleep_setting_leaves_no_marker() {
    let mut rig = Rig::mac_set_up();
    rig.platform.sudo = rig.stand_in("sudo", "exit 1");
    let marker = rig.marker();
    assert_eq!(
        hold::lid(&rig.platform, &marker).unwrap_err(),
        HoldError::Setting
    );
    assert!(!marker.exists());
}

#[test]
fn the_watcher_sets_sleep_back_when_the_followed_process_ends() {
    let mut rig = Rig::mac_set_up();
    let mut followed = short_lived();
    rig.platform.followed = followed.id();
    let held = hold::lid(&rig.platform, &rig.marker()).expect("the lid hold");
    followed.wait().unwrap();
    let marker = rig.marker();
    assert!(eventually(|| !marker.exists()));
    assert_eq!(rig.sudo_calls()[1], "sudo -n pmset -a disablesleep 0");
    assert_eq!(rig.sudo_calls().len(), 2);
    // The app is gone in this story: nothing else may set the setting back a second time.
    std::mem::forget(held);
}

#[test]
fn a_marker_left_by_an_earlier_run_sets_sleep_back_and_a_failure_keeps_the_marker() {
    let mut rig = Rig::mac_set_up();
    let marker = rig.marker();
    fs::write(&marker, "").unwrap();
    let working = rig.platform.sudo.clone();
    rig.platform.sudo = rig.stand_in("refusing-sudo", "exit 1");
    assert_eq!(
        hold::restore(&rig.platform, &marker),
        Err(HoldError::Setting)
    );
    assert!(marker.exists());
    rig.platform.sudo = working;
    hold::restore(&rig.platform, &marker).unwrap();
    assert_eq!(rig.sudo_calls(), ["sudo -n pmset -a disablesleep 0"]);
    assert!(!marker.exists());
}

#[test]
fn a_failed_new_hold_keeps_the_marker_of_a_setting_that_is_still_changed() {
    let mut rig = Rig::mac_set_up();
    let marker = rig.marker();
    fs::write(&marker, "").unwrap();
    rig.platform.pmset = rig.stand_in("pmset", "echo ' SleepDisabled 1'");
    rig.platform.sudo = rig.stand_in("sudo", "exit 1");
    assert_eq!(
        hold::lid(&rig.platform, &marker).unwrap_err(),
        HoldError::Setting
    );
    assert!(marker.exists());
}

#[test]
fn a_watcher_that_cannot_start_sets_the_setting_back_at_once() {
    let mut rig = Rig::mac_set_up();
    rig.platform.shell = "/nonexistent/anima-sh".into();
    let marker = rig.marker();
    assert_eq!(
        hold::lid(&rig.platform, &marker).unwrap_err(),
        HoldError::CouldNotStart
    );
    assert_eq!(
        rig.sudo_calls(),
        [
            "sudo -n pmset -a disablesleep 1",
            "sudo -n pmset -a disablesleep 0"
        ]
    );
    assert!(!marker.exists());
}

#[test]
fn when_the_setting_cannot_be_set_back_the_watcher_stays_to_do_it_at_the_end() {
    let mut rig = Rig::mac_set_up();
    let mut followed = short_lived();
    rig.platform.followed = followed.id();
    let marker = rig.marker();
    let log = rig.dir.path().join("calls");
    // `sudo` refuses the first set-back only.
    let once = rig.dir.path().join("refused-once");
    let body = format!(
        "case \"$*\" in *'disablesleep 0') [ -e '{once}' ] || {{ : > '{once}'; exit 1; }};; esac; echo \"sudo $*\" >> '{log}'",
        once = once.display(),
        log = log.display()
    );
    rig.platform.sudo = rig.stand_in("sudo", &body);
    let held = hold::lid(&rig.platform, &marker).expect("the lid hold");
    assert_eq!(
        hold::release_lid(&rig.platform, &marker, held),
        Err(HoldError::Setting)
    );
    assert!(marker.exists());
    followed.wait().unwrap();
    assert!(eventually(|| !marker.exists()));
    assert_eq!(
        rig.sudo_calls().last().unwrap(),
        "sudo -n pmset -a disablesleep 0"
    );
}

#[test]
fn a_hold_program_that_ignores_the_end_signal_is_killed() {
    let mut rig = Rig::new(Os::Linux);
    let mark = format!("anima-stubborn-test-{}", std::process::id());
    let body = format!("trap '' TERM; while :; do sleep 1; : {mark}; done");
    rig.platform.inhibit = rig.stand_in("inhibit", &format!("exec sh -c \"{body}\""));
    let held = hold::idle(&rig.platform).expect("a hold");
    assert!(eventually(|| running(&mark)));
    drop(held);
    assert!(eventually(|| !running(&mark)));
}
