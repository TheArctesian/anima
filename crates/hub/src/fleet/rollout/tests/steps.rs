use super::{device, machine, with_said};
use crate::fleet::rollout::{Reach, first_machine, step_size, steps};

#[test]
fn rol_023_a_step_has_at_most_a_quarter_of_the_machines_and_at_least_one() {
    let sizes: Vec<usize> = [1, 2, 3, 4, 7, 8, 9, 40]
        .into_iter()
        .map(step_size)
        .collect();
    assert_eq!(sizes, [1, 1, 1, 1, 1, 2, 2, 10]);
}

#[test]
fn rol_023_one_machine_is_first_and_alone_then_the_others_in_their_order() {
    let others: Vec<_> = (2..=10).map(device).collect();
    let all = steps(device(1), &others);
    let sizes: Vec<usize> = all.iter().map(Vec::len).collect();
    assert_eq!(sizes, [1, 2, 2, 2, 2, 1]);
    assert_eq!(all.concat(), (1..=10).map(device).collect::<Vec<_>>());
}

#[test]
fn rol_023_a_rollout_of_one_machine_is_one_step() {
    assert_eq!(steps(device(1), &[]), [[device(1)]]);
}

#[test]
fn rol_025_the_operator_can_select_a_different_first_machine_and_it_is_in_one_step_only() {
    let all = steps(device(3), &[device(1), device(2), device(3)]);
    assert_eq!(all, [vec![device(3)], vec![device(1)], vec![device(2)]]);
}

#[test]
fn rol_025_the_policy_takes_the_first_machine_that_is_connected_not_with_the_hub_and_runs_no_job() {
    let mut with_hub = machine(1);
    with_hub.with_hub = true;
    let mut away = machine(2);
    away.reach = Reach::NotConnected;
    let busy = with_said(machine(3), |said| said.busy = true);
    assert_eq!(
        first_machine(&[with_hub, away, busy, machine(4), machine(5)]),
        Some(device(4))
    );
}

#[test]
fn rol_025_the_policy_selects_none_when_no_machine_is_free_and_connected() {
    let mut with_hub = machine(1);
    with_hub.with_hub = true;
    assert_eq!(
        first_machine(&[with_hub, with_said(machine(2), |said| said.busy = true)]),
        None
    );
    assert_eq!(first_machine(&[]), None);
}
