use super::{NEW, OLD, machine, release, rolled_back, tried, updated, version, with_said};
use crate::fleet::rollout::{
    Aim, Cause, Instruction, Machine, Outcome, Reach, Release, Target, instruction, next_hop,
};

const MIDDLE: &str = "0.2.9";

fn forward(again: bool) -> Aim {
    Aim::Forward {
        version: version(NEW),
        again,
    }
}

fn told(machine: &Machine, again: bool) -> Option<Instruction> {
    instruction(machine, forward(again), &[release(NEW, "0.1.0")])
}

fn tell(text: &str, again: bool) -> Instruction {
    Instruction {
        version: version(text),
        again,
    }
}

#[test]
fn the_hub_tells_the_version_until_the_machine_has_it() {
    assert_eq!(told(&machine(1), false), Some(tell(NEW, false)));
    assert_eq!(told(&updated(machine(1), NEW), false), None);
    let mut checking = machine(1);
    checking.installed = Some(version(NEW));
    assert_eq!(told(&checking, false), None);
}

#[test]
fn rol_028_the_hub_tells_nothing_to_a_machine_that_is_held_set_off_or_has_no_update_function() {
    let mut held = machine(1);
    held.held = true;
    assert_eq!(told(&held, false), None);
    assert_eq!(
        told(&with_said(machine(1), |said| said.switch_on = false), false),
        None
    );
    let mut old_daemon = machine(1);
    old_daemon.said = None;
    assert_eq!(told(&old_daemon, false), None);
    let mut ended = machine(1);
    ended.reach = Reach::TokenEnded;
    assert_eq!(told(&ended, false), None);
}

#[test]
fn rol_028_a_busy_machine_gets_the_version_and_starts_by_itself_later() {
    assert_eq!(
        told(&with_said(machine(1), |said| said.busy = true), false),
        Some(tell(NEW, false))
    );
}

#[test]
fn rol_045_after_one_rollback_the_hub_gives_the_update_one_more_time() {
    assert_eq!(
        told(&rolled_back(machine(1), NEW, 1), false),
        Some(tell(NEW, false))
    );
}

#[test]
fn rol_032_after_the_second_rollback_the_hub_tells_nothing_until_try_again() {
    let twice = rolled_back(machine(1), NEW, 2);
    assert_eq!(told(&twice, false), None);
    assert_eq!(told(&twice, true), Some(tell(NEW, true)));
}

#[test]
fn rol_011_a_refused_release_is_not_told_again_until_try_again() {
    let refused = tried(machine(1), NEW, Outcome::Refused(Cause::FileMismatch), 1);
    assert_eq!(told(&refused, false), None);
    assert_eq!(told(&refused, true), Some(tell(NEW, true)));
}

#[test]
fn a_try_of_a_different_version_does_not_end_the_tries_of_this_one() {
    assert_eq!(
        told(&rolled_back(machine(1), "0.2.8", 2), false),
        Some(tell(NEW, false))
    );
}

#[test]
fn rol_041_a_machine_goes_to_the_newest_release_directly_when_the_record_permits() {
    let held = [release(MIDDLE, "0.1.0"), release(NEW, OLD)];
    assert_eq!(
        next_hop(version(OLD), Target::LinuxX8664, version(NEW), &held),
        Some(version(NEW))
    );
}

#[test]
fn rol_041_the_hub_gives_the_releases_between_in_order_when_the_newest_cannot_replace_the_version()
{
    let held = [release(MIDDLE, "0.1.0"), release(NEW, MIDDLE)];
    assert_eq!(
        instruction(&machine(1), forward(false), &held),
        Some(tell(MIDDLE, false))
    );
    let mut between = machine(1);
    between.installed = Some(version(MIDDLE));
    assert_eq!(
        instruction(&between, forward(false), &held),
        Some(tell(NEW, false))
    );
    assert_eq!(
        instruction(&rolled_back(machine(1), MIDDLE, 2), forward(false), &held),
        None
    );
}

#[test]
fn rol_041_no_version_is_told_when_no_held_release_can_replace_the_installed_one() {
    let linux = Target::LinuxX8664;
    assert_eq!(
        next_hop(version(OLD), linux, version(NEW), &[release(NEW, MIDDLE)]),
        None
    );
    assert_eq!(
        next_hop(
            version(OLD),
            linux,
            version(NEW),
            &[release(MIDDLE, "0.1.0")]
        ),
        None
    );
    assert_eq!(
        next_hop(version(NEW), linux, version(NEW), &[release(NEW, "0.1.0")]),
        None
    );
    assert_eq!(
        next_hop(
            version("0.3.1"),
            linux,
            version(NEW),
            &[release(NEW, "0.1.0")]
        ),
        None
    );
    let for_mac = Release {
        target: Target::DarwinUniversal,
        ..release(NEW, "0.1.0")
    };
    assert_eq!(
        next_hop(version(OLD), linux, version(NEW), &[for_mac]),
        None
    );
}

#[test]
fn rol_031_roll_back_tells_the_previous_version_until_the_machine_has_it() {
    let after = with_said(updated(machine(1), NEW), |said| {
        said.previous = Some(version(OLD));
    });
    assert_eq!(instruction(&after, Aim::Back, &[]), Some(tell(OLD, false)));
    assert_eq!(instruction(&updated(machine(1), NEW), Aim::Back, &[]), None);
    let back = with_said(machine(1), |said| said.previous = Some(version(OLD)));
    assert_eq!(instruction(&back, Aim::Back, &[]), None);
}
