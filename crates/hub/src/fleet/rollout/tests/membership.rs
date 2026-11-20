use super::{NEW, OLD, machine, release, version, with_said};
use crate::fleet::rollout::{LeftOut, Reach, Release, Target, place};

fn placed(machine: &super::Machine) -> Result<(), LeftOut> {
    place(machine, version(NEW), &[release(NEW, "0.1.0")])
}

#[test]
fn a_machine_with_a_lower_version_and_a_release_for_its_target_is_included() {
    assert_eq!(placed(&machine(1)), Ok(()));
}

#[test]
fn rol_039_a_machine_that_is_not_connected_and_a_busy_machine_are_included() {
    let mut away = machine(1);
    away.reach = Reach::NotConnected;
    assert_eq!(placed(&away), Ok(()));
    assert_eq!(
        placed(&with_said(machine(2), |said| said.busy = true)),
        Ok(())
    );
}

#[test]
fn rol_028_a_held_machine_and_a_machine_set_off_are_left_out_with_the_reason() {
    let mut held = machine(1);
    held.held = true;
    assert_eq!(placed(&held), Err(LeftOut::Held));
    assert_eq!(
        placed(&with_said(held, |said| said.switch_on = false)),
        Err(LeftOut::SetOff)
    );
}

#[test]
fn rol_043_rol_044_an_ended_token_comes_first_then_a_daemon_with_no_update_function() {
    let mut old_daemon = machine(1);
    old_daemon.said = None;
    assert_eq!(placed(&old_daemon), Err(LeftOut::ManualInstallation));
    old_daemon.reach = Reach::TokenEnded;
    assert_eq!(placed(&old_daemon), Err(LeftOut::EnrolAgain));
}

#[test]
fn a_machine_with_no_release_for_its_target_is_left_out() {
    let mac = with_said(machine(1), |said| {
        said.target = Some(Target::DarwinUniversal);
    });
    assert_eq!(placed(&mac), Err(LeftOut::NoRelease));
    assert_eq!(
        placed(&with_said(machine(2), |said| said.target = None)),
        Err(LeftOut::NoRelease)
    );
    let for_mac = Release {
        target: Target::DarwinUniversal,
        ..release(NEW, "0.1.0")
    };
    assert_eq!(place(&mac, version(NEW), &[for_mac]), Ok(()));
    let mut no_version = machine(3);
    no_version.installed = None;
    assert_eq!(placed(&no_version), Err(LeftOut::NoRelease));
}

#[test]
fn a_machine_that_has_the_version_or_a_higher_one_is_left_out() {
    let mut same = machine(1);
    same.installed = Some(version(NEW));
    assert_eq!(placed(&same), Err(LeftOut::UpToDate));
    same.installed = Some(version("0.3.1"));
    assert_eq!(placed(&same), Err(LeftOut::UpToDate));
    assert_eq!(
        place(&machine(2), version(OLD), &[release(OLD, "0.1.0")]),
        Err(LeftOut::UpToDate)
    );
}
