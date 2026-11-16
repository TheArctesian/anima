//! OD-E14, PRJ-035, PRO-010, PRJ-028: a workflow started by hand and a commit status each send
//! exactly the request of K-E1 Section 13.3 once, answer K1's shape, and leave one audit entry
//! and one event. A start drops the held run list; GitHub gives no run for it on API version
//! 2022-11-28, so the answer names none.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{answer, audits, body, events, sent};
use crate::{
    hub::start,
    projects::forge::{BASE, Forge, forge, reply, sha},
};

pub const DISPATCH: &str = "mutation($repository: ID!, $workflowId: ID!, $branch: String!, \
    $inputs: [WorkflowInputValue!]) { dispatchWorkflow(input: { repository: $repository, \
    workflowId: $workflowId, branch: $branch, inputs: $inputs }) { repository workflowId branch \
    acceptedAt } }";

pub const STATUS: &str = "mutation($repository: ID!, $sha: String!, $state: CommitStatusState!, \
    $context: String!, $description: String, $targetUrl: String) { setCommitStatus(input: { \
    repository: $repository, sha: $sha, state: $state, context: $context, description: \
    $description, targetUrl: $targetUrl }) { repository sha context state description \
    targetUrl createdAt } }";

/// The repository's workflows: `ci.yml` (31) active, `nightly.yml` (32) disabled.
pub fn workflows(forge: &Forge) {
    forge.ok(
        "/actions/workflows",
        &json!({ "total_count": 2, "workflows": [
            { "id": 31, "name": "CI", "path": ".github/workflows/ci.yml", "state": "active",
              "html_url": "https://github.com/octo/widgets/actions/workflows/ci.yml" },
            { "id": 32, "name": "Nightly", "path": ".github/workflows/nightly.yml",
              "state": "disabled_manually",
              "html_url": "https://github.com/octo/widgets/actions/workflows/nightly.yml" }
        ] }),
    );
}

/// GitHub accepts a start of workflow 31 as on API version 2022-11-28: 204, no body.
pub fn accepted(forge: &Forge) {
    let path = format!("{BASE}/actions/workflows/31/dispatches");
    (forge.kit.stub).reply_to("POST", &path, reply(204, &[], ""));
}

const LIST: &str =
    "query($repository: ID!) { workflowRuns(repository: $repository) { edges { cursor } } }";

#[sqlx::test(migrations = "./migrations")]
async fn prj_035_a_start_posts_the_branch_and_inputs_and_names_no_run(pool: PgPool) {
    let forge = forge(&pool).await;
    workflows(&forge);
    forge.ok(
        "/actions/runs",
        &json!({ "total_count": 0, "workflow_runs": [] }),
    );
    let listed = forge.ask(LIST, json!({})).await;
    assert!(listed["errors"].is_null(), "{listed}");
    accepted(&forge);
    let inputs = json!([{ "name": "target", "value": "staging" },
        { "name": "dry-run", "value": "" }]);

    let response = forge
        .ask(
            DISPATCH,
            json!({ "workflowId": "ci.yml", "branch": "release/1.2", "inputs": inputs }),
        )
        .await;

    assert_eq!(
        response["data"]["dispatchWorkflow"],
        json!({ "repository": forge.repository, "workflowId": "31", "branch": "release/1.2",
            "acceptedAt": start().to_rfc3339() }),
        "{response}"
    );
    let posts = sent(&forge, "POST", "/actions/workflows/31/dispatches");
    assert_eq!(posts.len(), 1, "the workflow found active, by its number");
    assert_eq!(
        body(&posts[0]),
        json!({ "ref": "release/1.2", "inputs": { "target": "staging", "dry-run": "" } })
    );
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 1);
    let change = &entries[0]["change"];
    assert_eq!(entries[0]["action"], "start");
    assert_eq!(
        change,
        &json!({ "function": "dispatch_workflow", "repository": "octo/widgets",
            "target": "workflow#31", "outcome": "done", "workflow": "31",
            "branch": "release/1.2", "inputs": 2 })
    );
    assert_eq!(
        events(&pool).await,
        [(
            "info".to_owned(),
            "Project widgets: dispatch_workflow on workflow#31, done.".to_owned()
        )]
    );
    let again = forge.ask(LIST, json!({})).await;
    assert!(again["errors"].is_null(), "{again}");
    assert_eq!(
        forge.heard("/actions/runs").len(),
        2,
        "the held run list was dropped"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_035_a_start_with_no_inputs_sends_an_empty_object(pool: PgPool) {
    let forge = forge(&pool).await;
    workflows(&forge);
    accepted(&forge);

    let response = forge
        .ask(DISPATCH, json!({ "workflowId": "31", "branch": "main" }))
        .await;

    assert_eq!(
        response["data"]["dispatchWorkflow"]["workflowId"], "31",
        "{response}"
    );
    let posts = sent(&forge, "POST", "/actions/workflows/31/dispatches");
    assert_eq!(body(&posts[0]), json!({ "ref": "main", "inputs": {} }));
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_010_a_commit_status_is_posted_under_anima_and_recorded(pool: PgPool) {
    let forge = forge(&pool).await;
    let commit = sha(9);
    let path = format!("/statuses/{commit}");
    answer(
        &forge,
        ("POST", &path),
        201,
        &json!({ "id": 1, "state": "success" }),
    );
    let url = "https://hub.anima.ing/projects/widgets/builds/7";

    let response = forge
        .ask(
            STATUS,
            json!({ "sha": commit.to_uppercase(), "state": "SUCCESS", "context": "anima/build",
                "description": "Built on mac-1", "targetUrl": url }),
        )
        .await;

    assert_eq!(
        response["data"]["setCommitStatus"],
        json!({ "repository": forge.repository, "sha": commit, "context": "anima/build",
            "state": "SUCCESS", "description": "Built on mac-1", "targetUrl": url,
            "createdAt": start().to_rfc3339() }),
        "{response}"
    );
    let posts = sent(&forge, "POST", &path);
    assert_eq!(posts.len(), 1);
    assert_eq!(
        body(&posts[0]),
        json!({ "state": "success", "context": "anima/build", "description": "Built on mac-1",
            "target_url": url })
    );
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["action"], "create");
    assert_eq!(
        entries[0]["change"],
        json!({ "function": "set_commit_status", "repository": "octo/widgets",
            "target": format!("commit#{commit}"), "outcome": "done", "sha": commit,
            "context": "anima/build", "state": "success" })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_010_a_status_with_no_description_or_address_sends_neither(pool: PgPool) {
    let forge = forge(&pool).await;
    let commit = sha(9);
    let path = format!("/statuses/{commit}");
    answer(&forge, ("POST", &path), 201, &json!({}));

    let response = forge
        .ask(
            STATUS,
            json!({ "sha": commit, "state": "PENDING", "context": "anima/test", "description": "" }),
        )
        .await;

    let node = &response["data"]["setCommitStatus"];
    assert_eq!(
        (&node["description"], &node["targetUrl"]),
        (&Value::Null, &Value::Null),
        "{response}"
    );
    let posts = sent(&forge, "POST", &path);
    assert_eq!(
        body(&posts[0]),
        json!({ "state": "pending", "context": "anima/test" })
    );
}
