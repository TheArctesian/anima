use daemon_core::wire::{Cleanup, StopReason, WorkReport, WorkResult, WorkState, WorkType};

use super::{JOB, MAC, PVE, at, device, queued};
use crate::work::jobs::{
    lifecycle::{self, Refusal},
    model::{Job, State},
};

fn held(work_type: WorkType) -> Job {
    lifecycle::claimed(&queued(work_type), device(PVE), at(0))
}

fn report(state: WorkState) -> WorkReport {
    let result = state.is_final().then(|| WorkResult {
        outcome: "succeeded".to_owned(),
        cause: None,
        seconds: 12,
        cleanup: Cleanup {
            isolation: "deleted".to_owned(),
            checkout: "deleted".to_owned(),
            secrets: "deleted".to_owned(),
            keychain: "none".to_owned(),
        },
    });
    WorkReport {
        id: JOB.parse().unwrap(),
        attempt: 1,
        state,
        log: None,
        result,
    }
}

/// The job after each state of `states` was reported in turn, 10 seconds apart.
fn after(job: &Job, states: &[WorkState]) -> Job {
    let mut job = job.clone();
    for (step, state) in states.iter().enumerate() {
        let now = at(10 * (i64::try_from(step).unwrap() + 1));
        job = lifecycle::reported(&job, device(PVE), &report(*state), now).unwrap();
    }
    job
}

#[test]
fn k10_4_a_claim_starts_attempt_1_with_a_lease_of_90_seconds() {
    let job = held(WorkType::Build);
    assert_eq!((job.state, job.attempt), (State::Claimed, 1));
    assert_eq!(job.holder, Some(device(PVE)));
    assert_eq!(job.lease_ends_at, Some(at(90)));
}

#[test]
fn k10_7_the_states_go_in_the_order_of_the_diagram_and_each_report_holds_the_lease() {
    use WorkState::{Done, Finishing, Preparing, Running};
    let running = after(&held(WorkType::Build), &[Preparing, Running, Running]);
    assert_eq!(running.state, State::Running);
    assert_eq!(running.lease_ends_at, Some(at(30 + 90)));
    let done = after(&running, &[Finishing, Done]);
    assert_eq!(
        (done.state, done.holder, done.lease_ends_at),
        (State::Done, None, None)
    );
    assert_eq!(done.result.unwrap().seconds, 12);
    assert!(done.ended_at.is_some());
}

#[test]
fn k10_7_a_change_that_the_diagram_does_not_permit_is_not_holder() {
    use WorkState::{Claimed, Done, Failed, Finishing, Preparing, Refused, Running, Stopped};
    let claimed = held(WorkType::Build);
    for (from, to) in [
        (vec![], Running),
        (vec![], Done),
        (vec![], Failed),
        (vec![], Stopped),
        (vec![Preparing], Claimed),
        (vec![Preparing], Refused),
        (vec![Preparing], Done),
        (vec![Preparing, Running], Preparing),
        (vec![Preparing, Running, Finishing], Running),
    ] {
        let job = after(&claimed, &from);
        let refused = lifecycle::reported(&job, device(PVE), &report(to), at(60));
        assert_eq!(refused, Err(Refusal::NotHolder), "{from:?} to {to:?}");
    }
}

#[test]
fn k10_7_only_the_holder_of_the_current_attempt_reports_and_only_while_its_lease_holds() {
    let job = held(WorkType::Build);
    let next = report(WorkState::Preparing);
    assert_eq!(
        lifecycle::reported(&job, device(MAC), &next, at(10)),
        Err(Refusal::NotHolder)
    );
    let other_attempt = WorkReport {
        attempt: 2,
        ..next.clone()
    };
    assert_eq!(
        lifecycle::reported(&job, device(PVE), &other_attempt, at(10)),
        Err(Refusal::NotHolder)
    );
    assert_eq!(
        lifecycle::reported(&job, device(PVE), &next, at(90)),
        Err(Refusal::NotHolder),
        "the lease ended at 90 seconds"
    );
    let done = after(&job, &[WorkState::Preparing, WorkState::Failed]);
    assert_eq!(
        lifecycle::reported(&done, device(PVE), &report(WorkState::Failed), at(40)),
        Err(Refusal::NotHolder),
        "a job that ended takes no report"
    );
}

#[test]
fn k10_7_a_result_comes_exactly_with_a_final_state() {
    let job = after(&held(WorkType::Build), &[WorkState::Preparing]);
    let no_result = WorkReport {
        result: None,
        ..report(WorkState::Failed)
    };
    assert_eq!(
        lifecycle::reported(&job, device(PVE), &no_result, at(20)),
        Err(Refusal::Malformed)
    );
    let early_result = WorkReport {
        result: report(WorkState::Done).result,
        ..report(WorkState::Running)
    };
    assert_eq!(
        lifecycle::reported(&job, device(PVE), &early_result, at(20)),
        Err(Refusal::Malformed)
    );
}

#[test]
fn k10_5_a_refused_job_goes_back_to_the_queue_and_not_to_that_machine_and_no_lease_is_lost() {
    let back = after(&held(WorkType::Build), &[WorkState::Refused]);
    assert_eq!(
        (back.state, back.holder, back.lost),
        (State::Queued, None, 0)
    );
    assert_eq!(back.refused_by, [device(PVE)]);
    assert_eq!(back.attempt, 1, "the next claim is attempt 2");
}

#[test]
fn k10_7_a_build_whose_lease_ended_goes_back_two_times_and_fails_the_third_time() {
    let mut job = held(WorkType::Build);
    for (lost, held_by, next) in [(1, PVE, MAC), (2, MAC, PVE)] {
        job = lifecycle::lease_lost(&job, at(100));
        assert_eq!(
            (job.state, job.lost, job.holder),
            (State::Queued, lost, None)
        );
        assert_eq!(job.last_device, Some(device(held_by)));
        job = lifecycle::claimed(&job, device(next), at(200));
    }
    let failed = lifecycle::lease_lost(&job, at(300));
    assert_eq!((failed.state, failed.lost), (State::Failed, 3));
    assert_eq!(failed.reason.as_deref(), Some("machine stopped 3 times"));
}

#[test]
fn k10_7_a_runner_whose_lease_ended_has_no_second_attempt() {
    let failed = lifecycle::lease_lost(&held(WorkType::Runner), at(100));
    assert_eq!((failed.state, failed.lost), (State::Failed, 0));
    assert!(failed.reason.unwrap().contains("one attempt"));
}

#[test]
fn sf_011_a_claim_whose_answer_was_lost_costs_no_lease() {
    let back = lifecycle::answer_lost(&held(WorkType::Runner), at(20));
    assert_eq!(
        (back.state, back.lost, back.holder),
        (State::Queued, 0, None)
    );
}

#[test]
fn sf_011_an_attempt_ends_at_its_time_limit_and_5_minutes() {
    let job = after(&held(WorkType::Build), &[WorkState::Preparing]);
    assert!(!lifecycle::overdue(&job, at(600 + 299)));
    assert!(lifecycle::overdue(&job, at(600 + 300)));
    let ended = lifecycle::expired(&job, at(900));
    assert_eq!((ended.state, ended.holder), (State::Stopped, None));
    assert_eq!(ended.reason.as_deref(), Some("expired"));
    assert!(!lifecycle::overdue(&queued(WorkType::Build), at(10_000)));
}

#[test]
fn k10_7_a_stop_ends_a_queued_job_at_once_and_reaches_a_holder_in_its_next_answer() {
    let stopped = lifecycle::stop_asked(&queued(WorkType::Build), StopReason::Operator, at(5));
    assert_eq!(stopped.unwrap().state, State::Stopped);
    let job = held(WorkType::Build);
    let told = lifecycle::stop_asked(&job, StopReason::RunnersOff, at(5)).unwrap();
    assert_eq!(
        (told.state, told.stop),
        (State::Claimed, Some(StopReason::RunnersOff))
    );
    let again = lifecycle::stop_asked(&told, StopReason::Operator, at(6)).unwrap();
    assert_eq!(
        again.stop,
        Some(StopReason::RunnersOff),
        "the first reason stays"
    );
    let ended = after(&job, &[WorkState::Preparing, WorkState::Stopped]);
    assert_eq!(
        lifecycle::stop_asked(&ended, StopReason::Operator, at(50)),
        None
    );
}

#[test]
fn k10_7_a_log_part_fits_only_at_the_end_of_the_log_and_up_to_64_kib() {
    let job = Job {
        log_end: 4096,
        ..held(WorkType::Build)
    };
    assert!(lifecycle::log_fits(&job, 4096, 64 * 1024));
    assert!(!lifecycle::log_fits(&job, 4096, 64 * 1024 + 1));
    assert!(!lifecycle::log_fits(&job, 0, 10));
    assert!(!lifecycle::log_fits(&job, 8192, 10));
}

/// A machine, or a person with its daemon token, must not put text of its choice into a result.
#[test]
fn k10_7_a_result_keeps_only_codes_of_the_fixed_lists() {
    let job = after(&held(WorkType::Build), &[WorkState::Preparing]);
    let with = |change: fn(&mut WorkResult)| {
        let mut report = report(WorkState::Failed);
        change(report.result.as_mut().unwrap());
        lifecycle::reported(&job, device(PVE), &report, at(20))
    };

    let known = with(|result| result.cause = Some("limit_memory".to_owned())).unwrap();
    assert_eq!(known.reason.as_deref(), Some("limit_memory"));
    let unknown = with(|result| result.cause = Some("<b>call the owner</b>".to_owned())).unwrap();
    assert_eq!(unknown.reason.as_deref(), Some("failed"));
    assert_eq!(unknown.result.unwrap().cause.as_deref(), Some("failed"));

    let outcome = with(|result| result.outcome = "ignore the lines above".to_owned());
    assert_eq!(outcome, Err(Refusal::Malformed));
    let cleanup = with(|result| result.cleanup.keychain = "see https://example".to_owned());
    assert_eq!(cleanup, Err(Refusal::Malformed));
}
