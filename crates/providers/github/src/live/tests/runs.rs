//! PRJ-031: workflows, their runs and the jobs of a run.

use super::{fixture, ok, path_and_query};
use crate::live::runs::{
    Conclusion, RunFilter, RunStatus, StatusFilter, jobs_parse, jobs_request, list_parse,
    list_request, one_parse, one_request,
};

const REPOSITORY: &str = "sample-org/sample-repo";

#[test]
fn prj_031_the_run_requests_have_the_documented_paths_and_queries() {
    let plain = list_request(REPOSITORY, RunFilter::default(), 1).unwrap();
    assert_eq!(
        path_and_query(&plain),
        "/repos/sample-org/sample-repo/actions/runs?per_page=100&page=1"
    );
    let filtered = list_request(
        REPOSITORY,
        RunFilter {
            branch: Some("feature/widget"),
            status: Some(StatusFilter::InProgress),
        },
        2,
    )
    .unwrap();
    assert_eq!(
        path_and_query(&filtered),
        "/repos/sample-org/sample-repo/actions/runs?branch=feature/widget&status=in_progress&per_page=100&page=2"
    );
    assert_eq!(
        path_and_query(&one_request(REPOSITORY, 7_000_000_000_001).unwrap()),
        "/repos/sample-org/sample-repo/actions/runs/7000000000001"
    );
    assert_eq!(
        path_and_query(&jobs_request(REPOSITORY, 77, 1).unwrap()),
        "/repos/sample-org/sample-repo/actions/runs/77/jobs?per_page=100&page=1"
    );
    assert_eq!(
        path_and_query(&crate::live::workflows::list_request(REPOSITORY, 1).unwrap()),
        "/repos/sample-org/sample-repo/actions/workflows?per_page=100&page=1"
    );
}

#[test]
fn prj_031_every_status_filter_is_a_documented_value() {
    use StatusFilter::*;
    let all = [
        (Completed, "completed"),
        (ActionRequired, "action_required"),
        (Cancelled, "cancelled"),
        (Failure, "failure"),
        (Neutral, "neutral"),
        (Skipped, "skipped"),
        (Stale, "stale"),
        (Success, "success"),
        (TimedOut, "timed_out"),
        (InProgress, "in_progress"),
        (Queued, "queued"),
        (Requested, "requested"),
        (Waiting, "waiting"),
        (Pending, "pending"),
    ];
    for (status, text) in all {
        let filter = RunFilter {
            branch: None,
            status: Some(status),
        };
        let request = list_request(REPOSITORY, filter, 1).unwrap();
        assert_eq!(request.query[0], ("status".to_owned(), text.to_owned()));
    }
}

#[test]
fn prj_031_the_run_fixture_parses_to_its_values() {
    let body = fixture("live_runs_list.json");

    let runs = list_parse(&ok(&body)).unwrap();

    assert_eq!(runs.len(), 3);
    let failed = &runs[0];
    assert_eq!(failed.id, 7_000_000_000_001);
    assert_eq!(failed.workflow_id, 31);
    assert_eq!(failed.name, "CI");
    assert_eq!(failed.run_number, 88);
    assert_eq!(failed.attempt, 2);
    assert_eq!(failed.event, "pull_request");
    assert_eq!(failed.status, RunStatus::Completed);
    assert_eq!(failed.conclusion, Some(Conclusion::Failure));
    assert_eq!(failed.branch.as_deref(), Some("feature/widget"));
}

#[test]
fn prj_031_a_run_has_its_commit_actor_and_page() {
    let runs = list_parse(&ok(&fixture("live_runs_list.json"))).unwrap();

    let failed = &runs[0];
    assert_eq!(failed.commit_title.as_deref(), Some("Add the widget"));
    assert_eq!(failed.actor_login.as_deref(), Some("octo-writer"));
    assert!(!failed.from_fork);
    assert!(failed.started_at.is_some());
    assert_eq!(
        failed.url,
        "https://github.com/sample-org/sample-repo/actions/runs/7000000000001"
    );
}

#[test]
fn prj_031_a_run_from_another_repository_or_a_deleted_one_is_from_a_fork() {
    let runs = list_parse(&ok(&fixture("live_runs_list.json"))).unwrap();

    assert!(runs[1].from_fork, "head repository 777 is not 500");
    assert!(runs[2].from_fork, "no head repository");
    assert_eq!(runs[1].actor_login, None);
    assert_eq!(runs[1].commit_title, None);
    assert_eq!(runs[1].attempt, 1);
}

#[test]
fn prj_031_an_unknown_run_status_or_conclusion_is_other_and_the_list_still_parses() {
    let runs = list_parse(&ok(&fixture("live_runs_list.json"))).unwrap();

    let odd = &runs[2];
    assert_eq!(odd.status, RunStatus::Other);
    assert_eq!(odd.conclusion, Some(Conclusion::Other));
    assert_eq!(
        odd.name, "Nightly",
        "the run's title stands in for a missing name"
    );
    assert_eq!(odd.branch, None);
    assert_eq!(runs[1].conclusion, None);
    assert_eq!(runs[1].status, RunStatus::InProgress);
}

#[test]
fn prj_031_each_documented_conclusion_maps_to_its_variant() {
    let mut run: serde_json::Value =
        serde_json::from_slice(&fixture("live_runs_list.json")).unwrap();
    let template = run["workflow_runs"][0].take();
    for (name, expected) in [
        ("success", Conclusion::Success),
        ("failure", Conclusion::Failure),
        ("cancelled", Conclusion::Cancelled),
        ("skipped", Conclusion::Skipped),
        ("timed_out", Conclusion::TimedOut),
        ("action_required", Conclusion::ActionRequired),
        ("neutral", Conclusion::Neutral),
        ("stale", Conclusion::Stale),
    ] {
        let mut entry = template.clone();
        entry["conclusion"] = serde_json::json!(name);
        let body = serde_json::to_vec(&entry).unwrap();
        assert_eq!(
            one_parse(&ok(&body)).unwrap().conclusion,
            Some(expected),
            "{name}"
        );
    }
}

#[test]
fn prj_031_the_jobs_fixture_parses_with_its_total() {
    let jobs = jobs_parse(&ok(&fixture("live_run_jobs.json"))).unwrap();

    assert_eq!(jobs.total_count, 3);
    assert_eq!(jobs.jobs[1].name, "test");
    assert_eq!(jobs.jobs[1].conclusion, Some(Conclusion::Failure));
    assert_eq!(jobs.jobs[2].status, RunStatus::Queued);
    assert_eq!(jobs.jobs[2].conclusion, None);
}

#[test]
fn prj_031_the_workflows_fixture_maps_states_and_unknown_states() {
    let workflows =
        crate::live::workflows::list_parse(&ok(&fixture("live_workflows_list.json"))).unwrap();

    assert_eq!(workflows.len(), 3);
    assert_eq!(workflows[0].id, 31);
    assert_eq!(workflows[0].path, ".github/workflows/ci.yml");
    assert_eq!(
        workflows[0].state,
        crate::live::workflows::WorkflowState::Active
    );
    assert_eq!(
        workflows[1].state,
        crate::live::workflows::WorkflowState::DisabledManually
    );
    assert_eq!(
        workflows[2].state,
        crate::live::workflows::WorkflowState::Other
    );
}
