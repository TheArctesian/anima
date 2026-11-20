use super::{NEW, OLD, machine, rolled_back, tried, updated, version, with_said};
use crate::fleet::rollout::{Aimed, Cause, Machine, Outcome, Reach, State, state};

fn aimed_now() -> Aimed {
    Aimed {
        now: Some(version(NEW)),
        newest: Some(version(NEW)),
        ..Aimed::default()
    }
}

fn refused(machine: Machine) -> Machine {
    tried(machine, NEW, Outcome::Refused(Cause::RecordMismatch), 1)
}

#[test]
fn the_first_four_rows_come_before_each_result_in_their_order() {
    let mut all = refused(machine(1));
    all.held = true;
    assert_eq!(state(&all, aimed_now()), State::Held);
    let off = with_said(all, |said| said.switch_on = false);
    assert_eq!(state(&off, aimed_now()), State::SetOff);
    let mut old_daemon = off;
    old_daemon.said = None;
    assert_eq!(state(&old_daemon, aimed_now()), State::ManualInstallation);
    old_daemon.reach = Reach::TokenEnded;
    assert_eq!(state(&old_daemon, aimed_now()), State::EnrolAgain);
}

#[test]
fn a_refusal_and_a_rollback_of_the_told_version_come_before_the_connection() {
    let mut away = refused(machine(1));
    away.reach = Reach::NotConnected;
    assert_eq!(
        state(&away, aimed_now()),
        State::Refused(Cause::RecordMismatch)
    );
    let mut back = rolled_back(machine(1), NEW, 1);
    back.reach = Reach::NotConnected;
    assert_eq!(state(&back, aimed_now()), State::RolledBack(Cause::NoCheck));
}

#[test]
fn a_rollback_of_the_version_that_the_hub_told_before_still_shows() {
    let before = Aimed {
        before: Some(version(NEW)),
        newest: Some(version(NEW)),
        ..Aimed::default()
    };
    assert_eq!(
        state(&rolled_back(machine(1), NEW, 2), before),
        State::RolledBack(Cause::NoCheck)
    );
    assert_eq!(state(&refused(machine(1)), before), State::Available);
    assert_eq!(
        state(&rolled_back(machine(1), "0.2.8", 2), aimed_now()),
        State::Updating
    );
}

#[test]
fn rol_039_a_machine_that_is_told_a_version_and_is_not_connected_waits() {
    let mut away = machine(1);
    away.reach = Reach::NotConnected;
    assert_eq!(state(&away, aimed_now()), State::NotConnected);
    assert_eq!(
        state(
            &away,
            Aimed {
                newest: Some(version(NEW)),
                ..Aimed::default()
            }
        ),
        State::Available
    );
}

#[test]
fn updating_then_checking_then_up_to_date() {
    assert_eq!(state(&machine(1), aimed_now()), State::Updating);
    let mut checking = machine(1);
    checking.installed = Some(version(NEW));
    assert_eq!(state(&checking, aimed_now()), State::Checking);
    assert_eq!(
        state(&updated(machine(1), NEW), aimed_now()),
        State::UpToDate
    );
}

#[test]
fn a_machine_whose_step_did_not_start_waits_for_its_step() {
    let waiting = Aimed {
        waits_for_step: true,
        newest: Some(version(NEW)),
        ..Aimed::default()
    };
    assert_eq!(state(&machine(1), waiting), State::WaitingForStep);
}

#[test]
fn an_update_is_available_only_for_a_higher_verified_release() {
    let holds = |text: &str| Aimed {
        newest: Some(version(text)),
        ..Aimed::default()
    };
    assert_eq!(state(&machine(1), holds(NEW)), State::Available);
    assert_eq!(state(&machine(1), holds(OLD)), State::UpToDate);
    assert_eq!(state(&machine(1), holds("0.2.2")), State::UpToDate);
    assert_eq!(state(&machine(1), Aimed::default()), State::UpToDate);
}
