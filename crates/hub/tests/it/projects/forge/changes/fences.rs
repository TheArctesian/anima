//! K-E1 Sections 13.3 and 13.6, PRJ-034, PRJ-035, PRO-010, PRO-013: a run control, a start or
//! a status the hub refuses before sending sends nothing to GitHub and records nothing. A run
//! that has not finished is not run again; a finished one is not cancelled; a fork's run, a run
//! older than 30 days and a run whose head moved are not run again; a start takes at most 25
//! inputs, each name once, of an active workflow; a status is under `anima/`, links only below
//! the hub and says at most 140 characters. The words never hold a value given (LOG-007).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    audits,
    controls::{asked, control, run_in},
    starts::{DISPATCH, STATUS, workflows},
};
use crate::{
    hub::error,
    projects::forge::{BASE, Forge, forge, sha},
};

const MARK: &str = "REFUSED-VALUE-4417";

/// Requests below the repository that would change something.
fn changes_sent(forge: &Forge) -> usize {
    forge.kit.stub.with(|s| {
        s.heard
            .iter()
            .filter(|h| h.method != "GET" && h.path.starts_with(BASE))
            .count()
    })
}

/// The fields a `VALIDATION` error names.
fn fields(response: &Value) -> Vec<String> {
    assert_eq!(error(response).1, "VALIDATION", "{response}");
    let listed = response["errors"][0]["extensions"]["fields"]
        .as_array()
        .unwrap_or_else(|| panic!("no fields: {response}"));
    listed
        .iter()
        .map(|f| f["field"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_a_run_in_the_wrong_state_is_refused_before_sending(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/actions/runs/1", &run_in(1, "in_progress", None, 1));
    forge.ok(
        "/actions/runs/2",
        &run_in(2, "completed", Some("failure"), 1),
    );
    let mut forked = run_in(3, "completed", Some("failure"), 1);
    forked["head_repository"] = json!({ "id": 99 });
    forge.ok("/actions/runs/3", &forked);
    let mut old = run_in(4, "completed", Some("failure"), 1);
    old["created_at"] = json!("2030-04-05T11:59:59Z");
    forge.ok("/actions/runs/4", &old);
    forge.ok(
        "/actions/runs/5",
        &run_in(5, "completed", Some("success"), 1),
    );
    let cases = [
        (
            "rerunWorkflowRun",
            asked(1),
            "The run has not finished. Only a finished run can run again.",
        ),
        (
            "cancelWorkflowRun",
            asked(2),
            "The run has finished. There is nothing to cancel.",
        ),
        (
            "rerunWorkflowRun",
            asked(3),
            "The run is from a fork. Anima does not run it again (PRO-013).",
        ),
        (
            "rerunFailedWorkflowJobs",
            asked(3),
            "The run is from a fork. Anima does not run it again (PRO-013).",
        ),
        (
            "rerunWorkflowRun",
            asked(4),
            "The run started more than 30 days ago. GitHub runs it again only within 30 days.",
        ),
        (
            "rerunFailedWorkflowJobs",
            asked(5),
            "The run has no failed, cancelled or timed out jobs to run again.",
        ),
        (
            "cancelWorkflowRun",
            json!({ "runId": "1", "sha": sha(77) }),
            "The run's commit is not the one you saw. Read the run again.",
        ),
    ];
    for (root, variables, words) in cases {
        let response = forge.ask(&control(root), variables).await;
        assert_eq!(
            error(&response),
            (words.to_owned(), "CONFLICT".to_owned()),
            "{root}"
        );
    }
    assert_eq!(changes_sent(&forge), 0, "nothing was sent");
    assert!(audits(&pool).await.is_empty(), "nothing was recorded");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_bad_arguments_and_a_missing_run_send_nothing(pool: PgPool) {
    let forge = forge(&pool).await;
    let bad = forge
        .ask(
            &control("rerunWorkflowRun"),
            json!({ "runId": format!("{MARK}1"), "sha": MARK }),
        )
        .await;
    assert_eq!(fields(&bad), ["runId", "expectedHeadSha"]);
    assert!(!bad.to_string().contains(MARK), "{bad}");
    let missing = forge.ask(&control("cancelWorkflowRun"), asked(404)).await;
    assert_eq!(
        error(&missing),
        (
            "GitHub has no such run, or the App may not read it.".to_owned(),
            "NOT_FOUND".to_owned()
        )
    );
    assert_eq!(changes_sent(&forge), 0);
    assert_eq!(audits(&pool).await, Vec::<Value>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_035_a_start_the_hub_cannot_send_is_refused_before_sending(pool: PgPool) {
    let forge = forge(&pool).await;
    workflows(&forge);
    let many: Vec<Value> = (0..26)
        .map(|n| json!({ "name": format!("input-{n}"), "value": MARK }))
        .collect();
    let twice = json!([{ "name": "target", "value": MARK }, { "name": "target", "value": MARK }]);
    let cases = [
        (
            json!({ "workflowId": "ci.yml", "branch": "main", "inputs": many }),
            vec!["inputs"],
        ),
        (
            json!({ "workflowId": "ci.yml", "branch": "main", "inputs": twice }),
            vec!["inputs"],
        ),
        (
            json!({ "workflowId": "ci.yml", "branch": "main",
                "inputs": [{ "name": format!("{MARK} name"), "value": "x" }] }),
            vec!["inputs"],
        ),
        (
            json!({ "workflowId": "ci.yml", "branch": "main",
                "inputs": [{ "name": "x", "value": format!("{MARK}{}", "v".repeat(1025)) }] }),
            vec!["inputs"],
        ),
        (
            json!({ "workflowId": format!("../{MARK}"), "branch": format!("{MARK}..x") }),
            vec!["workflowId", "branch"],
        ),
    ];
    for (variables, named) in cases {
        let response = forge.ask(DISPATCH, variables.clone()).await;
        assert_eq!(fields(&response), named, "{variables}");
        assert!(!response.to_string().contains(MARK), "{response}");
    }
    let inactive = forge
        .ask(
            DISPATCH,
            json!({ "workflowId": "nightly.yml", "branch": "main" }),
        )
        .await;
    assert_eq!(
        error(&inactive),
        (
            "The workflow is not active on GitHub. Enable it there first.".to_owned(),
            "CONFLICT".to_owned()
        )
    );
    let unknown = forge
        .ask(
            DISPATCH,
            json!({ "workflowId": "deploy.yml", "branch": "main" }),
        )
        .await;
    assert_eq!(error(&unknown).1, "NOT_FOUND", "{unknown}");
    assert_eq!(changes_sent(&forge), 0);
    assert_eq!(audits(&pool).await, Vec::<Value>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_010_a_status_outside_its_fence_is_refused_before_sending(pool: PgPool) {
    let forge = forge(&pool).await;
    let commit = sha(9);
    let status = |context: &str, extra: Value| {
        let mut variables = json!({ "sha": commit, "state": "SUCCESS", "context": context });
        variables
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        variables
    };
    let cases = [
        (status("ci/build", json!({})), vec!["context"]),
        (status("anima/Build", json!({})), vec!["context"]),
        (
            status(&format!("anima/{}", "a".repeat(41)), json!({})),
            vec!["context"],
        ),
        (
            status(
                "anima/build",
                json!({ "targetUrl": format!("https://evil.example/{MARK}") }),
            ),
            vec!["targetUrl"],
        ),
        (
            status(
                "anima/build",
                json!({ "targetUrl": format!("https://hub.anima.ing.evil.example/{MARK}") }),
            ),
            vec!["targetUrl"],
        ),
        (
            status(
                "anima/build",
                json!({ "targetUrl": format!("https://hub.anima.ing@evil.example/{MARK}") }),
            ),
            vec!["targetUrl"],
        ),
        (
            status(
                "anima/build",
                json!({ "description": format!("{MARK}{}", "d".repeat(141 - MARK.len())) }),
            ),
            vec!["description"],
        ),
        (
            json!({ "sha": MARK, "state": "SUCCESS", "context": "anima/build" }),
            vec!["sha"],
        ),
    ];
    for (variables, named) in cases {
        let response = forge.ask(STATUS, variables.clone()).await;
        assert_eq!(fields(&response), named, "{variables}");
        assert!(!response.to_string().contains(MARK), "{response}");
    }
    let fits = status("anima/build", json!({ "description": "d".repeat(140) }));
    let answer = format!("{BASE}/statuses/{commit}");
    (forge.kit.stub).reply_to(
        "POST",
        &answer,
        crate::projects::forge::reply(201, &[], "{}"),
    );
    let response = forge.ask(STATUS, fits).await;
    assert!(
        response["errors"].is_null(),
        "140 characters fit: {response}"
    );
    assert_eq!(changes_sent(&forge), 1, "only the status that fits");
    assert_eq!(audits(&pool).await.len(), 1);
}
