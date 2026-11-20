use chrono::TimeDelta;

use super::{NEW, at, device, machine, rolled_back, tried, updated, version, with_said};
use crate::fleet::rollout::{
    Cause, Machine, Next, Outcome, Pace, Reach, Rollout, Standing, Step, next, standing,
};

/// A rollout of the machines 1 to 5 in the steps [1], [2, 3], [4, 5], of which `started` began.
fn rollout(started: usize, pace: Pace) -> Rollout {
    let step = |index: usize, machines: &[u8]| Step {
        machines: machines.iter().copied().map(device).collect(),
        started_at: (index < started).then(|| at(0)),
        good_at: None,
    };
    Rollout {
        version: version(NEW),
        steps: vec![step(0, &[1]), step(1, &[2, 3]), step(2, &[4, 5])],
        pace,
        continued_after: Vec::new(),
    }
}

fn fleet() -> Vec<Machine> {
    (1..=5).map(machine).collect()
}

fn good(mut rollout: Rollout, index: usize, seconds: i64) -> Rollout {
    rollout.steps[index].good_at = Some(at(seconds));
    rollout
}

#[test]
fn rol_023_a_new_rollout_starts_with_its_first_step() {
    assert_eq!(
        next(&rollout(0, Pace::PRESSED), &fleet(), at(0)),
        Next::Start(0)
    );
}

#[test]
fn rol_026_a_step_is_good_only_when_each_of_its_machines_passed_its_check() {
    let mut machines = fleet();
    assert_eq!(
        next(&rollout(1, Pace::PRESSED), &machines, at(60)),
        Next::Wait
    );
    machines[0].installed = Some(version(NEW));
    assert_eq!(standing(&machines[0], version(NEW)), Standing::Pending);
    assert_eq!(
        next(&rollout(1, Pace::PRESSED), &machines, at(60)),
        Next::Wait
    );
    machines[0] = updated(machine(1), NEW);
    assert_eq!(
        next(&rollout(1, Pace::PRESSED), &machines, at(60)),
        Next::Good(0)
    );
    machines[1] = updated(machine(2), NEW);
    assert_eq!(
        next(&good(rollout(2, Pace::PRESSED), 0, 60), &machines, at(900)),
        Next::Wait
    );
}

#[test]
fn rol_026_the_next_step_starts_10_minutes_after_a_good_step_and_not_earlier() {
    let mut machines = fleet();
    machines[0] = updated(machine(1), NEW);
    let after_first = good(rollout(1, Pace::PRESSED), 0, 60);
    assert_eq!(next(&after_first, &machines, at(60 + 599)), Next::Wait);
    assert_eq!(next(&after_first, &machines, at(60 + 600)), Next::Start(1));
}

#[test]
fn rol_047_an_automatic_rollout_waits_1_hour_after_the_first_machine_and_10_minutes_after_each_other_step()
 {
    assert_eq!(Pace::AUTOMATIC.after_first, TimeDelta::hours(1));
    let mut machines = fleet();
    for machine in &mut machines[..3] {
        *machine = updated(*machine, NEW);
    }
    let after_first = good(rollout(1, Pace::AUTOMATIC), 0, 0);
    assert_eq!(next(&after_first, &machines, at(3599)), Next::Wait);
    assert_eq!(next(&after_first, &machines, at(3600)), Next::Start(1));
    let after_second = good(good(rollout(2, Pace::AUTOMATIC), 0, 0), 1, 4000);
    assert_eq!(next(&after_second, &machines, at(4599)), Next::Wait);
    assert_eq!(next(&after_second, &machines, at(4600)), Next::Start(2));
}

#[test]
fn rol_027_a_rollback_or_a_refusal_stops_the_rollout_and_the_operator_can_continue() {
    let mut machines = fleet();
    machines[0] = updated(machine(1), NEW);
    machines[2] = rolled_back(machine(3), NEW, 1);
    let mut stopped = good(rollout(2, Pace::PRESSED), 0, 0);
    assert_eq!(next(&stopped, &machines, at(9000)), Next::Stop(device(3)));
    machines[1] = updated(machine(2), NEW);
    assert_eq!(next(&stopped, &machines, at(9000)), Next::Stop(device(3)));
    stopped.continued_after.push(device(3));
    assert_eq!(next(&stopped, &machines, at(9000)), Next::Good(1));
    machines[2] = tried(machine(3), NEW, Outcome::Refused(Cause::RecordSignature), 1);
    stopped.continued_after.clear();
    assert_eq!(next(&stopped, &machines, at(9000)), Next::Stop(device(3)));
}

#[test]
fn rol_027_a_failure_in_a_step_that_did_not_start_does_not_stop_the_rollout() {
    let mut machines = fleet();
    machines[4] = rolled_back(machine(5), NEW, 2);
    assert_eq!(
        next(&rollout(1, Pace::PRESSED), &machines, at(60)),
        Next::Wait
    );
}

#[test]
fn rol_039_a_step_does_not_wait_for_a_machine_that_is_not_connected() {
    let mut machines = fleet();
    machines[0] = updated(machine(1), NEW);
    machines[1].reach = Reach::NotConnected;
    machines[2] = updated(machine(3), NEW);
    assert_eq!(standing(&machines[1], version(NEW)), Standing::Left);
    assert_eq!(
        next(&good(rollout(2, Pace::PRESSED), 0, 0), &machines, at(9000)),
        Next::Good(1)
    );
}

#[test]
fn rol_039_the_rollout_can_finish_without_the_machines_that_are_not_connected() {
    let mut machines = fleet();
    for machine in &mut machines[..3] {
        *machine = updated(*machine, NEW);
    }
    machines[3].reach = Reach::NotConnected;
    machines[4].reach = Reach::NotConnected;
    let last = good(good(rollout(3, Pace::PRESSED), 0, 0), 1, 700);
    assert_eq!(next(&last, &machines, at(2000)), Next::Good(2));
    assert_eq!(
        next(&good(last, 2, 2000), &machines, at(2000)),
        Next::Finish
    );
}

#[test]
fn rol_028_a_step_does_not_wait_for_a_machine_that_the_hub_tells_nothing() {
    let mut held = machine(1);
    held.held = true;
    let mut gone = machine(1);
    gone.said = None;
    let mut ended = machine(1);
    ended.reach = Reach::TokenEnded;
    let off = with_said(machine(1), |said| said.switch_on = false);
    for left in [held, gone, ended, off] {
        assert_eq!(standing(&left, version(NEW)), Standing::Left);
        assert_eq!(
            next(&rollout(1, Pace::PRESSED), &[left], at(60)),
            Next::Good(0)
        );
    }
    assert_eq!(next(&rollout(1, Pace::PRESSED), &[], at(60)), Next::Good(0));
}

#[test]
fn rol_028_a_step_waits_for_a_busy_machine_that_is_connected() {
    let busy = with_said(machine(1), |said| said.busy = true);
    assert_eq!(standing(&busy, version(NEW)), Standing::Pending);
    assert_eq!(
        next(&rollout(1, Pace::PRESSED), &[busy], at(86_400)),
        Next::Wait
    );
}

#[test]
fn rol_040_a_machine_with_a_higher_version_passed_and_a_rollback_comes_before_the_connection() {
    let mut newer = machine(1);
    newer.installed = Some(version("0.3.1"));
    assert_eq!(standing(&newer, version(NEW)), Standing::Passed);
    let mut away = rolled_back(machine(2), NEW, 1);
    away.reach = Reach::NotConnected;
    assert_eq!(standing(&away, version(NEW)), Standing::Failed);
}

#[test]
fn a_rollout_with_no_step_is_complete() {
    let mut empty = rollout(0, Pace::PRESSED);
    empty.steps.clear();
    assert_eq!(next(&empty, &[], at(0)), Next::Finish);
}
