//! PRJ-031, PRJ-034, PRJ-035: a repository's workflows by name, its runs in K1's shape and
//! paged to the end, one run, and the count of a run's jobs.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{forge, nodes, run};

const RUNS: &str = "query($repository: ID!, $branch: String, $status: WorkflowRunStatus, \
    $first: Int, $after: String) { workflowRuns(repository: $repository, branch: $branch, \
    status: $status, first: $first, after: $after) { edges { node { repository id workflowId \
    name runNumber attempt event status conclusion branch headSha commitTitle actorLogin \
    fromFork url rerunnable failedJobsRerunnable cancellable } } \
    pageInfo { hasNextPage endCursor } read { stale } } }";

const ONE: &str = "query($repository: ID!, $runId: ID!) { workflowRun(repository: \
    $repository, runId: $runId) { id status jobs { total succeeded failed running queued \
    other failing partial } } }";

#[sqlx::test(migrations = "./migrations")]
async fn prj_035_workflows_come_by_name(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok(
        "/actions/workflows",
        &json!({ "total_count": 3, "workflows": [
            { "id": 33, "name": "zeta", "path": ".github/workflows/z.yml", "state": "paused_by_aliens", "html_url": "https://github.com/z" },
            { "id": 31, "name": "CI", "path": ".github/workflows/ci.yml", "state": "active", "html_url": "https://github.com/c" },
            { "id": 32, "name": "Nightly", "path": ".github/workflows/n.yml", "state": "disabled_manually", "html_url": "https://github.com/n" }
        ] }),
    );
    let query = "query($repository: ID!) { workflows(repository: $repository) { id name path active url } }";
    let response = forge.ask(query, json!({})).await;
    assert_eq!(
        response["data"]["workflows"],
        json!([
            { "id": "31", "name": "CI", "path": ".github/workflows/ci.yml", "active": true, "url": "https://github.com/c" },
            { "id": "32", "name": "Nightly", "path": ".github/workflows/n.yml", "active": false, "url": "https://github.com/n" },
            { "id": "33", "name": "zeta", "path": ".github/workflows/z.yml", "active": false, "url": "https://github.com/z" }
        ])
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_runs_come_back_in_k1_shape(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/actions/runs",
        super::ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_runs_list.json"
        )),
    );

    let response = forge.ask(RUNS, json!({})).await;
    let runs = nodes(&response, "workflowRuns");
    assert_eq!(
        runs[0],
        json!({
            "repository": forge.repository, "id": "7000000000001", "workflowId": "31",
            "name": "CI", "runNumber": 88, "attempt": 2, "event": "pull_request",
            "status": "COMPLETED", "conclusion": "FAILURE", "branch": "feature/widget",
            "headSha": "1".repeat(40), "commitTitle": "Add the widget",
            "actorLogin": "octo-writer", "fromFork": false,
            "url": runs[0]["url"], "rerunnable": true, "failedJobsRerunnable": true,
            "cancellable": false
        })
    );
    let fork = &runs[1];
    assert_eq!(
        [
            &fork["status"],
            &fork["conclusion"],
            &fork["fromFork"],
            &fork["rerunnable"],
            &fork["cancellable"]
        ],
        [
            &json!("IN_PROGRESS"),
            &Value::Null,
            &json!(true),
            &json!(false),
            &json!(true)
        ]
    );
    let odd = &runs[2];
    assert_eq!(
        [
            &odd["name"],
            &odd["status"],
            &odd["conclusion"],
            &odd["branch"],
            &odd["attempt"]
        ],
        [
            &json!("Nightly"),
            &json!("IN_PROGRESS"),
            &Value::Null,
            &Value::Null,
            &json!(1)
        ],
        "a status GitHub may add is not read as finished"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_a_filtered_list_ends_at_1000_runs(pool: PgPool) {
    let forge = forge(&pool).await;
    let page: Vec<Value> = (1..=100).map(run).collect();
    forge.ok(
        "/actions/runs",
        &json!({ "total_count": 5000, "workflow_runs": page }),
    );

    let mut after = Value::Null;
    let mut rows = 0;
    loop {
        let variables =
            json!({ "branch": "main", "status": "COMPLETED", "first": 50, "after": after });
        let response = forge.ask(RUNS, variables).await;
        rows += nodes(&response, "workflowRuns").len();
        let end = &response["data"]["workflowRuns"]["pageInfo"];
        if end["hasNextPage"] == false {
            break;
        }
        after = end["endCursor"].clone();
    }
    assert_eq!(rows, 1000);
    let heard = forge.heard("/actions/runs");
    assert_eq!(heard.len(), 10, "{heard:?}");
    assert!(
        heard
            .iter()
            .all(|h| h.contains("branch=main") && h.contains("status=completed"))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_a_branch_with_a_control_character_is_validation(pool: PgPool) {
    let forge = forge(&pool).await;
    for branch in ["", "main\u{0}", &"b".repeat(256)] {
        let response = forge.ask(RUNS, json!({ "branch": branch })).await;
        let errors = &response["errors"][0]["extensions"];
        assert_eq!(
            (&errors["code"], &errors["fields"][0]["field"]),
            (&json!("VALIDATION"), &json!("branch"))
        );
    }
    assert_eq!(forge.live_reads(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_one_run_with_its_jobs_counted(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/actions/runs/41", &run(41));
    forge.reply(
        "/actions/runs/41/jobs",
        super::ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_run_jobs.json"
        )),
    );

    let response = forge.ask(ONE, json!({ "runId": "41" })).await;
    assert_eq!(
        response["data"]["workflowRun"],
        json!({ "id": "41", "status": "COMPLETED", "jobs": { "total": 3, "succeeded": 1,
            "failed": 1, "running": 0, "queued": 1, "other": 0, "failing": ["test"],
            "partial": false } })
    );
    let missing = forge.ask(ONE, json!({ "runId": "42" })).await;
    assert_eq!(missing, json!({ "data": { "workflowRun": null } }));
    for bad in ["0", "-1", "12a", "9007199254740993", ""] {
        let refused = forge.ask(ONE, json!({ "runId": bad })).await;
        assert_eq!(
            refused["errors"][0]["extensions"]["fields"][0]["field"], "runId",
            "{bad}"
        );
    }
}
