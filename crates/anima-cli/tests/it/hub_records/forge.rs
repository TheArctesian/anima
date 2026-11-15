//! `anima pr`, `anima issue` and `anima workflow` against the scripted hub (PRJ-030 to PRJ-035,
//! CMD-026, CMD-031).

mod history;
mod issue_changes;
mod merge;
mod merge_many;
mod run_controls;
mod start_and_status;

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};
use crate::kit::forge::{
    FULL_NAME, REPOSITORY, SHA, comment, connection, issue, project, pull_request, run, workflow,
};

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-forge-tests")
}

/// Runs `anima <args> --hub <origin>`.
async fn ask(hub: &FakeHub, args: &[&str]) -> crate::kit::Outcome {
    let mut all = args.to_vec();
    all.extend(["--hub", &hub.origin]);
    signed_in().run(&all).await
}

fn variables(hub: &FakeHub, operation: &str) -> Vec<Value> {
    hub.seen()
        .into_iter()
        .filter(|seen| seen.operation == operation)
        .map(|seen| seen.variables)
        .collect()
}

#[tokio::test]
async fn prj_031_pr_list_names_the_repository_by_full_name_and_prints_a_table_and_json() {
    let hub = FakeHub::start().await;
    project(&hub);
    let pulls = [pull_request(7, "Fix the thing", None)];
    hub.answer(
        "PullRequestList",
        data(connection("pullRequests", &pulls, Some("next-1"))),
    );
    let table = ask(&hub, &["pr", "list", FULL_NAME, "--state", "all"]).await;
    assert_eq!(table.code, 0, "{table:?}");
    assert!(table.stdout.contains("Fix the thing"), "{table:?}");
    assert!(table.stdout.contains("fix-it -> main"), "{table:?}");
    assert!(table.stderr.contains("--after next-1"), "{table:?}");
    assert!(table.stderr.contains("--state all"), "{table:?}");
    let sent = &variables(&hub, "PullRequestList")[0];
    assert_eq!(sent["repository"], REPOSITORY);
    assert_eq!(sent["state"], "ALL");
    assert_eq!(sent["first"], 20);

    let lookups = variables(&hub, "ProjectList").len();
    let json = ask(&hub, &["pr", "list", REPOSITORY, "--json", "--fresh"]).await;
    assert_eq!(json.json()["edges"][0]["node"]["number"], 7);
    let sent = &variables(&hub, "PullRequestList")[1];
    assert_eq!(sent["state"], "OPEN");
    assert_eq!(sent["fresh"], true);
    assert_eq!(
        variables(&hub, "ProjectList").len(),
        lookups,
        "an ID needs no lookup"
    );
}

#[tokio::test]
async fn prj_033_pr_list_checks_asks_for_mergeability_checks_and_merge_methods() {
    let hub = FakeHub::start().await;
    let pulls = [pull_request(7, "Fix the thing", None)];
    let mut page = connection("pullRequests", &pulls, None);
    page["pullRequests"]["mergeMethods"] = json!(["SQUASH", "MERGE"]);
    hub.answer("PullRequestListChecked", data(page));
    let run = ask(&hub, &["pr", "list", REPOSITORY, "--checks"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(variables(&hub, "PullRequestListChecked").len(), 1);
    assert_eq!(variables(&hub, "PullRequestList").len(), 0);
    assert!(run.stdout.contains("yes"), "{run:?}");
    assert!(run.stdout.contains("PASSED"), "{run:?}");
    assert!(
        run.stdout.contains("merge methods: SQUASH, MERGE"),
        "{run:?}"
    );
}

#[tokio::test]
async fn cmd_026_pr_list_all_follows_the_cursor_with_the_largest_page() {
    let hub = FakeHub::start().await;
    let first = [pull_request(8, "Newer", None)];
    let second = [pull_request(7, "Older", None)];
    hub.answer(
        "PullRequestList",
        data(connection("pullRequests", &first, Some("p2"))),
    );
    hub.answer(
        "PullRequestList",
        data(connection("pullRequests", &second, None)),
    );
    let run = ask(&hub, &["pr", "list", REPOSITORY, "--all", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["edges"].as_array().map(Vec::len), Some(2));
    let sent = variables(&hub, "PullRequestList");
    assert_eq!(
        (&sent[0]["first"], &sent[1]["after"]),
        (&json!(50), &json!("p2"))
    );
}

#[tokio::test]
async fn prj_030_pr_get_prints_the_pull_request_with_its_comments() {
    let hub = FakeHub::start().await;
    let pull = pull_request(7, "Fix the thing", Some("Closes #3"));
    hub.answer("PullRequestGet", data(json!({ "pullRequest": pull })));
    let comments = [comment("901", "Looks good to me")];
    hub.answer(
        "IssueComments",
        data(connection("issueComments", &comments, None)),
    );
    let table = ask(&hub, &["pr", "get", REPOSITORY, "7", "--comments"]).await;
    assert_eq!(table.code, 0, "{table:?}");
    for shown in ["Closes #3", "CLEAN", "3 passed", "Looks good to me", SHA] {
        assert!(table.stdout.contains(shown), "{shown}: {table:?}");
    }
    let sent = &variables(&hub, "PullRequestGet")[0];
    assert_eq!(
        (&sent["repository"], &sent["number"]),
        (&json!(REPOSITORY), &json!(7))
    );
    let sent = &variables(&hub, "IssueComments")[0];
    assert_eq!((&sent["number"], &sent["first"]), (&json!(7), &json!(30)));

    let json = ask(
        &hub,
        &["pr", "get", REPOSITORY, "7", "--comments", "--json"],
    )
    .await;
    assert_eq!(json.json()["comments"]["edges"][0]["node"]["id"], "901");
    assert_eq!(json.json()["number"], 7);
}

#[tokio::test]
async fn prj_030_pr_get_for_a_number_that_is_not_there_exits_3() {
    let hub = FakeHub::start().await;
    hub.answer("PullRequestGet", data(json!({ "pullRequest": null })));
    let run = ask(&hub, &["pr", "get", REPOSITORY, "99"]).await;
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(variables(&hub, "IssueComments").len(), 0);
}

#[tokio::test]
async fn prj_031_issue_list_sends_the_state_and_prints_the_labels() {
    let hub = FakeHub::start().await;
    project(&hub);
    let issues = [issue(7, "It breaks", None)];
    hub.answer("IssueList", data(connection("issues", &issues, None)));
    let run = ask(
        &hub,
        &[
            "issue", "list", FULL_NAME, "--state", "closed", "--limit", "5",
        ],
    )
    .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("It breaks"), "{run:?}");
    assert!(run.stdout.contains("help wanted"), "{run:?}");
    let sent = &variables(&hub, "IssueList")[0];
    assert_eq!(sent["repository"], REPOSITORY);
    assert_eq!(
        (&sent["state"], &sent["first"]),
        (&json!("CLOSED"), &json!(5))
    );
    let json = ask(&hub, &["issue", "list", REPOSITORY, "--json"]).await;
    assert_eq!(
        json.json()["edges"][0]["node"]["labels"],
        json!(["help wanted"])
    );
}

#[tokio::test]
async fn prj_030_issue_get_prints_the_issue_and_its_comments() {
    let hub = FakeHub::start().await;
    let found = issue(7, "It breaks", Some("Steps: run it"));
    hub.answer("IssueGet", data(json!({ "issue": found })));
    let comments = [comment("901", "Same here"), comment("902", "Fixed in #8")];
    let (older, newer) = comments.split_at(1);
    for (page, next) in [(older, Some("c2")), (newer, None)] {
        hub.answer(
            "IssueComments",
            data(connection("issueComments", page, next)),
        );
    }
    let run = ask(
        &hub,
        &[
            "issue",
            "get",
            REPOSITORY,
            "7",
            "--comments",
            "--all-comments",
        ],
    )
    .await;
    assert_eq!(run.code, 0, "{run:?}");
    for shown in ["Steps: run it", "maint", "Same here", "Fixed in #8"] {
        assert!(run.stdout.contains(shown), "{shown}: {run:?}");
    }
    assert_eq!(variables(&hub, "IssueGet")[0]["number"], 7);
    assert_eq!(variables(&hub, "IssueComments")[0]["first"], 100);
    assert_eq!(variables(&hub, "IssueComments")[1]["after"], "c2");

    let hub = FakeHub::start().await;
    hub.answer("IssueGet", data(json!({ "issue": null })));
    let missing = ask(&hub, &["issue", "get", REPOSITORY, "99"]).await;
    assert_eq!(missing.code, 3, "{missing:?}");
}

#[tokio::test]
async fn prj_035_workflow_list_prints_the_workflows() {
    let hub = FakeHub::start().await;
    hub.answer("WorkflowList", data(json!({ "workflows": [workflow()] })));
    let run = ask(&hub, &["workflow", "list", REPOSITORY]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains(".github/workflows/ci.yml"), "{run:?}");
    assert_eq!(variables(&hub, "WorkflowList")[0]["repository"], REPOSITORY);
    let json = ask(&hub, &["workflow", "list", REPOSITORY, "--json"]).await;
    assert_eq!(json.json(), json!([workflow()]));
}

#[tokio::test]
async fn prj_034_workflow_runs_filters_by_branch_and_status_and_shows_the_commit() {
    let hub = FakeHub::start().await;
    let runs = [run("5001", "Break the build")];
    for operation in ["WorkflowRunList", "WorkflowRunListJobs"] {
        hub.answer(operation, data(connection("workflowRuns", &runs, None)));
    }
    let table = ask(
        &hub,
        &[
            "workflow",
            "runs",
            REPOSITORY,
            "--branch",
            "main",
            "--status",
            "in-progress",
            "--jobs",
        ],
    )
    .await;
    assert_eq!(table.code, 0, "{table:?}");
    for shown in [
        "5001",
        "Break the build",
        "01234567",
        "FAILURE",
        "3 passed, 1 failed",
    ] {
        assert!(table.stdout.contains(shown), "{shown}: {table:?}");
    }
    let sent = &variables(&hub, "WorkflowRunListJobs")[0];
    assert_eq!(
        (&sent["branch"], &sent["status"]),
        (&json!("main"), &json!("IN_PROGRESS"))
    );
    assert_eq!(sent["first"], 20);
    let json = ask(&hub, &["workflow", "runs", REPOSITORY, "--json"]).await;
    assert_eq!(json.json()["edges"][0]["node"]["id"], "5001");
    assert_eq!(variables(&hub, "WorkflowRunList").len(), 1);
}

#[tokio::test]
async fn prj_034_workflow_run_shows_one_run_with_its_jobs() {
    let hub = FakeHub::start().await;
    hub.answer(
        "WorkflowRunGet",
        data(json!({ "workflowRun": run("5001", "Break it") })),
    );
    let table = ask(&hub, &["workflow", "run", REPOSITORY, "5001"]).await;
    assert_eq!(table.code, 0, "{table:?}");
    assert!(table.stdout.contains("test (linux)"), "{table:?}");
    assert!(table.stdout.contains("Break it"), "{table:?}");
    let sent = &variables(&hub, "WorkflowRunGet")[0];
    assert_eq!(
        (&sent["repository"], &sent["runId"]),
        (&json!(REPOSITORY), &json!("5001"))
    );
    let json = ask(&hub, &["workflow", "run", REPOSITORY, "5001", "--json"]).await;
    assert_eq!(json.json()["jobs"]["failed"], 1);

    let hub = FakeHub::start().await;
    hub.answer("WorkflowRunGet", data(json!({ "workflowRun": null })));
    let missing = ask(&hub, &["workflow", "run", REPOSITORY, "1"]).await;
    assert_eq!(missing.code, 3, "{missing:?}");
}

#[tokio::test]
async fn prj_016_an_unknown_repository_name_exits_3_and_sends_no_read() {
    let hub = FakeHub::start().await;
    project(&hub);
    let run = ask(&hub, &["pr", "list", "nobody/nothing"]).await;
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(variables(&hub, "PullRequestList").len(), 0);
}
