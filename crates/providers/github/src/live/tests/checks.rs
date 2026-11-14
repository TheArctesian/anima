//! PRJ-032: check runs and the combined status of a ref, counted together.

use super::{fixture, ok, path_and_query};
use crate::live::checks::{
    CheckRuns, ChecksState, CombinedStatus, StatusState, check_runs_parse, check_runs_request,
    count, status_parse, status_request,
};
use crate::live::runs::{Conclusion, RunStatus};

const REPOSITORY: &str = "sample-org/sample-repo";

fn runs(name: &str) -> CheckRuns {
    check_runs_parse(&ok(&fixture(name))).unwrap()
}

fn statuses(name: &str) -> CombinedStatus {
    status_parse(&ok(&fixture(name))).unwrap()
}

#[test]
fn prj_032_the_check_requests_have_the_documented_paths_and_queries() {
    let sha = "1".repeat(40);
    assert_eq!(
        path_and_query(&check_runs_request(REPOSITORY, &sha, 2).unwrap()),
        format!("/repos/sample-org/sample-repo/commits/{sha}/check-runs?per_page=100&page=2")
    );
    assert_eq!(
        path_and_query(&status_request(REPOSITORY, "heads/feature/a b", 1).unwrap()),
        "/repos/sample-org/sample-repo/commits/heads/feature/a%20b/status?per_page=100&page=1"
    );
}

#[test]
fn prj_032_the_check_run_and_status_fixtures_parse() {
    let check_runs = runs("live_check_runs.json");
    assert_eq!(check_runs.total_count, 6);
    assert_eq!(check_runs.runs[1].name, "lint");
    assert_eq!(check_runs.runs[1].status, RunStatus::Completed);
    assert_eq!(check_runs.runs[1].conclusion, Some(Conclusion::Failure));
    assert_eq!(check_runs.runs[3].conclusion, None);

    let combined = statuses("live_combined_status.json");
    assert_eq!(combined.total_count, 3);
    assert_eq!(combined.statuses[1].context, "ci/deploy-preview");
    assert_eq!(combined.statuses[1].state, StatusState::Error);
}

#[test]
fn prj_032_mixed_check_runs_and_statuses_are_counted_together_with_the_failing_names() {
    let counts = count(
        &runs("live_check_runs.json"),
        &statuses("live_combined_status.json"),
    );

    // runs: build passed; lint failed; docs skipped; e2e running; slow timed out; mystery other.
    // statuses: legacy passed; deploy-preview error; queue pending.
    assert_eq!(counts.passed, 2);
    assert_eq!(counts.failed, 3);
    assert_eq!(counts.pending, 2);
    assert_eq!(counts.other, 2);
    assert_eq!(counts.failing, ["lint", "slow", "ci/deploy-preview"]);
    assert_eq!(counts.state, ChecksState::Failed);
    assert!(!counts.partial);
}

#[test]
fn prj_032_a_combined_status_with_no_statuses_adds_nothing_and_is_not_pending() {
    let only_runs = count(
        &runs("live_check_runs.json"),
        &statuses("live_combined_status_empty.json"),
    );
    assert_eq!(only_runs.pending, 1, "only the running check run");

    let passing = CheckRuns {
        total_count: 1,
        runs: vec![crate::live::checks::CheckRun {
            name: "build".to_owned(),
            status: RunStatus::Completed,
            conclusion: Some(Conclusion::Success),
        }],
    };
    let counts = count(&passing, &statuses("live_combined_status_empty.json"));
    assert_eq!(counts.state, ChecksState::Passed);
    assert_eq!(counts.pending, 0);
}

#[test]
fn prj_032_a_commit_with_no_checks_and_no_statuses_is_none() {
    let counts = count(
        &runs("live_check_runs_empty.json"),
        &statuses("live_combined_status_empty.json"),
    );

    assert_eq!(counts.state, ChecksState::None);
    assert_eq!(
        (counts.passed, counts.failed, counts.pending, counts.other),
        (0, 0, 0, 0)
    );
    assert_eq!(counts.failing, Vec::<String>::new());
    assert!(!counts.partial);
}

#[test]
fn prj_032_the_overall_state_is_failed_then_pending_then_passed() {
    let one = |status: RunStatus, conclusion: Option<Conclusion>| CheckRuns {
        total_count: 1,
        runs: vec![crate::live::checks::CheckRun {
            name: "x".to_owned(),
            status,
            conclusion,
        }],
    };
    let none = statuses("live_combined_status_empty.json");

    let pending = count(&one(RunStatus::Queued, None), &none);
    assert_eq!(pending.state, ChecksState::Pending);
    let skipped = count(&one(RunStatus::Completed, Some(Conclusion::Skipped)), &none);
    assert_eq!(
        skipped.state,
        ChecksState::Passed,
        "nothing failed or waits"
    );
    let failed = count(
        &one(RunStatus::Completed, Some(Conclusion::ActionRequired)),
        &statuses("live_combined_status.json"),
    );
    assert_eq!(failed.state, ChecksState::Failed);
}

#[test]
fn prj_032_more_checks_than_given_is_partial() {
    let mut check_runs = runs("live_check_runs.json");
    check_runs.total_count = 140;
    let combined = statuses("live_combined_status_empty.json");

    assert!(count(&check_runs, &combined).partial);

    let mut many_statuses = statuses("live_combined_status.json");
    many_statuses.total_count = 101;
    assert!(count(&runs("live_check_runs_empty.json"), &many_statuses).partial);
}

#[test]
fn prj_032_at_most_ten_failing_names_each_cut_to_a_hundred_characters() {
    let long = "x".repeat(300);
    let failing = CheckRuns {
        total_count: 12,
        runs: (0..12)
            .map(|n| crate::live::checks::CheckRun {
                name: if n == 0 {
                    long.clone()
                } else {
                    format!("check-{n}")
                },
                status: RunStatus::Completed,
                conclusion: Some(Conclusion::Failure),
            })
            .collect(),
    };

    let counts = count(&failing, &statuses("live_combined_status_empty.json"));

    assert_eq!(counts.failed, 12);
    assert_eq!(counts.failing.len(), 10);
    assert_eq!(counts.failing[0].chars().count(), 100);
}

#[test]
fn prj_032_an_unknown_status_state_counts_as_other() {
    let body = br#"{"state":"x","sha":"s","total_count":1,"statuses":[{"context":"c","state":"strange"}]}"#;

    let combined = status_parse(&ok(body)).unwrap();

    assert_eq!(combined.statuses[0].state, StatusState::Other);
    let counts = count(&runs("live_check_runs_empty.json"), &combined);
    assert_eq!(counts.other, 1);
}
