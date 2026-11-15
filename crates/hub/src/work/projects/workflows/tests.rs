//! The run-control rule over every status, conclusion, fork and age (PRJ-034, PRO-013;
//! decisions item 10), and the workflow a start names (PRJ-035).

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use provider_github::live::workflows::{Workflow, WorkflowState};
use source_exchange::{CommitSha, WorkflowId};

use super::{
    controls::{Barred, Control, RunState, decide},
    dispatch::{Found, find},
    views::{WorkflowConclusion as C, WorkflowRunStatus as S},
};

const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

fn state(status: S, conclusion: Option<C>, from_fork: bool, age: TimeDelta) -> RunState {
    RunState {
        status,
        conclusion,
        head_sha: HEAD.to_owned(),
        from_fork,
        created_at: now() - age,
        attempt: 1,
    }
}

fn head() -> CommitSha {
    CommitSha::parse(HEAD).unwrap()
}

const ALL_CONCLUSIONS: [Option<C>; 10] = [
    None,
    Some(C::Success),
    Some(C::Failure),
    Some(C::Cancelled),
    Some(C::Skipped),
    Some(C::TimedOut),
    Some(C::ActionRequired),
    Some(C::Neutral),
    Some(C::Stale),
    Some(C::Other),
];

/// What the rule must answer for one cell of the table, by the words of PRJ-034 and the
/// rulings: run again needs a completed run that is not a fork's and not older than 30 days;
/// the failed jobs need that and a failing conclusion; cancel needs a run that has not ended.
fn expected(control: Control, run: &RunState) -> Result<(), Barred> {
    let completed = run.status == S::Completed;
    let too_old = now() - run.created_at > TimeDelta::days(30);
    let again = if !completed {
        Err(Barred::NotFinished)
    } else if run.from_fork {
        Err(Barred::FromFork)
    } else if too_old {
        Err(Barred::TooOld)
    } else {
        Ok(())
    };
    let failing = matches!(
        run.conclusion,
        Some(C::Failure | C::Cancelled | C::TimedOut)
    );
    match control {
        Control::Rerun => again,
        Control::RerunFailedJobs => again.and(if failing {
            Ok(())
        } else {
            Err(Barred::NoFailedJobs)
        }),
        Control::Cancel if completed => Err(Barred::Finished),
        Control::Cancel => Ok(()),
    }
}

#[test]
fn prj_034_the_rule_over_status_conclusion_fork_and_age() {
    let ages = [
        TimeDelta::zero(),
        TimeDelta::days(30),
        TimeDelta::days(30) + TimeDelta::seconds(1),
        TimeDelta::days(400),
    ];
    let runs: Vec<RunState> = [S::Queued, S::InProgress, S::Completed]
        .into_iter()
        .flat_map(|status| ALL_CONCLUSIONS.map(|conclusion| (status, conclusion)))
        .flat_map(|(status, conclusion)| [false, true].map(|fork| (status, conclusion, fork)))
        .flat_map(|(status, conclusion, fork)| ages.map(|age| state(status, conclusion, fork, age)))
        .collect();
    let mut cells = 0;
    for run in &runs {
        for control in [Control::Rerun, Control::RerunFailedJobs, Control::Cancel] {
            assert_eq!(
                decide(control, run, &head(), now()),
                expected(control, run),
                "{control:?} {run:?}"
            );
            cells += 1;
        }
    }
    assert_eq!(cells, 3 * 10 * 2 * 4 * 3);
}

#[test]
fn prj_034_named_cases_read_as_the_rulings_say() {
    let day = TimeDelta::days(1);
    let failed = state(S::Completed, Some(C::Failure), false, day);
    assert_eq!(decide(Control::Rerun, &failed, &head(), now()), Ok(()));
    assert_eq!(
        decide(Control::RerunFailedJobs, &failed, &head(), now()),
        Ok(())
    );
    assert_eq!(
        decide(Control::Cancel, &failed, &head(), now()),
        Err(Barred::Finished)
    );
    let running = state(S::InProgress, None, false, day);
    assert_eq!(
        decide(Control::Rerun, &running, &head(), now()),
        Err(Barred::NotFinished)
    );
    assert_eq!(decide(Control::Cancel, &running, &head(), now()), Ok(()));
    let forked_running = state(S::Queued, None, true, day);
    assert_eq!(
        decide(Control::Cancel, &forked_running, &head(), now()),
        Ok(()),
        "a fork's run can be cancelled: that stops code"
    );
    let succeeded = state(S::Completed, Some(C::Success), false, day);
    assert_eq!(
        decide(Control::RerunFailedJobs, &succeeded, &head(), now()),
        Err(Barred::NoFailedJobs)
    );
}

#[test]
fn prj_034_a_head_other_than_the_one_seen_is_refused_first() {
    let other = CommitSha::parse(&"f".repeat(40)).unwrap();
    for status in [S::Queued, S::Completed] {
        let run = state(status, Some(C::Failure), true, TimeDelta::days(90));
        for control in [Control::Rerun, Control::RerunFailedJobs, Control::Cancel] {
            assert_eq!(decide(control, &run, &other, now()), Err(Barred::HeadMoved));
        }
    }
}

#[test]
fn prj_034_github_runs_a_run_again_at_most_50_times() {
    let mut run = state(S::Completed, Some(C::Failure), false, TimeDelta::hours(1));
    run.attempt = 49;
    assert_eq!(decide(Control::Rerun, &run, &head(), now()), Ok(()));
    run.attempt = 50;
    for control in [Control::Rerun, Control::RerunFailedJobs] {
        assert_eq!(
            decide(control, &run, &head(), now()),
            Err(Barred::TooManyAttempts)
        );
    }
}

fn workflow(id: u64, path: &str, state: WorkflowState) -> Workflow {
    Workflow {
        id,
        name: "CI".to_owned(),
        path: path.to_owned(),
        state,
        url: "https://github.com/octo/widgets/actions".to_owned(),
    }
}

#[test]
fn prj_035_a_start_names_a_workflow_by_number_or_file_name() {
    let workflows = [
        workflow(31, ".github/workflows/ci.yml", WorkflowState::Active),
        workflow(
            32,
            ".github/workflows/nightly.yaml",
            WorkflowState::DisabledManually,
        ),
    ];
    let by = |text: &str| find(&workflows, &WorkflowId::parse(text).unwrap());
    assert_eq!(by("31"), Found::Active(&workflows[0]));
    assert_eq!(by("ci.yml"), Found::Active(&workflows[0]));
    assert_eq!(by("32"), Found::Inactive);
    assert_eq!(by("nightly.yaml"), Found::Inactive);
    assert_eq!(by("33"), Found::Missing);
    assert_eq!(
        by("i.yml"),
        Found::Missing,
        "the whole file name, not its end"
    );
}
