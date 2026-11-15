//! OD-E14, PRJ-034, PRJ-028: run again, run the failed jobs again and cancel each read the run
//! fresh, send exactly the request of K-E1 Section 13.3 once, return the run read back from
//! GitHub, leave one audit entry and one event, and drop the held run list. Every answer GitHub
//! gives reaches the client as its K7 code and fixed words.

use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{answer, audits, events, sent};
use crate::{
    hub::{error, start},
    projects::forge::{BASE, Forge, forge, ok, reply, run, sha},
};

/// The K1 document of a run control `root`, with `$runId` and `$sha`.
pub fn control(root: &str) -> String {
    format!(
        "mutation($repository: ID!, $runId: ID!, $sha: String!) {{ {root}(repository: \
         $repository, runId: $runId, expectedHeadSha: $sha) {{ repository id status conclusion \
         attempt headSha fromFork rerunnable failedJobsRerunnable cancellable }} }}"
    )
}

/// Run `id` as GitHub has it: `status`, `conclusion`, the attempt, the head `sha(id)`.
pub fn run_in(id: u64, status: &str, conclusion: Option<&str>, attempt: u32) -> Value {
    let mut found = run(id);
    found["status"] = json!(status);
    found["conclusion"] = json!(conclusion);
    found["run_attempt"] = json!(attempt);
    found
}

/// The variables of a control of run `id`, as the operator saw its head.
pub fn asked(id: u64) -> Value {
    json!({ "runId": id.to_string(), "sha": sha(id) })
}

const LIST: &str =
    "query($repository: ID!) { workflowRuns(repository: $repository) { edges { cursor } } }";

/// Reads the run list once, so a held answer is there to drop.
async fn listed(forge: &Forge) {
    forge.ok(
        "/actions/runs",
        &json!({ "total_count": 0, "workflow_runs": [] }),
    );
    let response = forge.ask(LIST, json!({})).await;
    assert!(response["errors"].is_null(), "{response}");
}

/// GitHub has run `id` as `before` for the next read, and as `after` from then on.
fn script(forge: &Forge, id: u64, before: &Value, after: &Value) {
    let path = format!("{BASE}/actions/runs/{id}");
    (forge.kit.stub).reply_once("GET", &path, ok(&before.to_string()));
    forge.ok(&format!("/actions/runs/{id}"), after);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_each_control_posts_once_returns_the_run_and_is_recorded(pool: PgPool) {
    let forge = forge(&pool).await;
    let failed = run_in(5, "completed", Some("failure"), 1);
    let queued = run_in(5, "queued", None, 2);
    let running = run_in(6, "in_progress", None, 1);
    let cancelled = run_in(6, "completed", Some("cancelled"), 1);
    let cases = [
        (
            "rerunWorkflowRun",
            5,
            "rerun",
            201,
            "rerun_workflow_run",
            "start",
            &failed,
            &queued,
        ),
        (
            "rerunFailedWorkflowJobs",
            5,
            "rerun-failed-jobs",
            201,
            "rerun_failed_workflow_jobs",
            "start",
            &failed,
            &queued,
        ),
        (
            "cancelWorkflowRun",
            6,
            "cancel",
            202,
            "cancel_workflow_run",
            "cancel",
            &running,
            &cancelled,
        ),
    ];
    for (n, (root, id, tail, status, function, action, before, after)) in
        cases.into_iter().enumerate()
    {
        listed(&forge).await;
        let held = forge.heard("/actions/runs").len();
        // Past the five seconds in which even a fresh read reuses a held answer.
        forge.clock.advance(10);
        script(&forge, id, before, after);
        let post = format!("/actions/runs/{id}/{tail}");
        answer(&forge, ("POST", &post), status, &json!({}));

        let response = forge.ask(&control(root), asked(id)).await;

        let node = &response["data"][root];
        assert_eq!(node["id"], json!(id.to_string()), "{response}");
        assert_eq!(node["repository"], json!(forge.repository));
        assert_eq!(node["headSha"], json!(sha(id)));
        let read_back = if root == "cancelWorkflowRun" {
            ("COMPLETED", json!("CANCELLED"), 1)
        } else {
            ("QUEUED", Value::Null, 2)
        };
        assert_eq!(
            (
                node["status"].as_str().unwrap(),
                node["conclusion"].clone(),
                node["attempt"].as_i64().unwrap()
            ),
            read_back,
            "{root}: the run as GitHub has it after the change"
        );
        let posts = sent(&forge, "POST", &post);
        assert_eq!(posts.len(), 1, "{root} sent once");
        assert!(posts[0].body.is_empty(), "{root} sends no body");
        let entries = audits(&pool).await;
        assert_eq!(entries.len(), n + 1, "one entry each");
        let entry = &entries[n];
        assert_eq!(entry["action"], action, "{root}");
        assert_eq!(entry["token_kind"], "login");
        assert_eq!(entry["change"]["function"], function);
        assert_eq!(entry["change"]["target"], format!("run#{id}"));
        assert_eq!(entry["change"]["run"], id);
        assert_eq!(entry["change"]["outcome"], "done");
        assert_eq!(
            events(&pool).await[n],
            (
                "info".to_owned(),
                format!("Project widgets: {function} on run#{id}, done.")
            )
        );
        let response = forge.ask(LIST, json!({})).await;
        assert!(response["errors"].is_null(), "{response}");
        assert_eq!(
            forge.heard("/actions/runs").len(),
            held + 1,
            "{root}: the held run list was dropped"
        );
        let reads = forge.heard(&format!("/actions/runs/{id}")).len();
        assert!(reads >= 2, "{root}: read before and after");
    }
}

/// A status, its headers, the words, the code and the recorded outcome.
type Case<'a> = (
    u16,
    &'a [(&'static str, &'a str)],
    &'static str,
    &'static str,
    &'static str,
);

const UNKNOWN: &str =
    "GitHub did not answer. The change may have been made: look before you try again.";
const GITHUB_TEXT: &str = "GITHUB-OWN-TEXT-7731";

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_each_answer_to_a_cancel_has_its_code_words_and_record(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/actions/runs/6", &run_in(6, "in_progress", None, 1));
    let reset = (start() + TimeDelta::minutes(15)).timestamp().to_string();
    let limit = [
        ("x-ratelimit-remaining", "0"),
        ("x-ratelimit-reset", reset.as_str()),
    ];
    let cases: [Case<'_>; 6] = [
        (
            409,
            &[],
            "GitHub refused it: the run is not in a state that allows this. Read it again.",
            "CONFLICT",
            "refused",
        ),
        (
            403,
            &[],
            "The GitHub App is not permitted to do that. Check its permissions on GitHub.",
            "FORBIDDEN",
            "refused",
        ),
        (
            404,
            &[],
            "GitHub has no such run, or the App may not change it.",
            "NOT_FOUND",
            "refused",
        ),
        (
            422,
            &[],
            "GitHub refused a value of the change. Nothing was changed.",
            "VALIDATION",
            "refused",
        ),
        (500, &[], UNKNOWN, "UNAVAILABLE", "unknown"),
        (
            403,
            &limit,
            "GitHub limits requests until 12:15.",
            "UNAVAILABLE",
            "limited",
        ),
    ];
    let path = format!("{BASE}/actions/runs/6/cancel");
    let said = json!({ "message": GITHUB_TEXT }).to_string();
    for (n, (status, headers, words, code, outcome)) in cases.into_iter().enumerate() {
        (forge.kit.stub).reply_to("POST", &path, reply(status, headers, &said));
        forge.clock.advance(2);
        let response = forge.ask(&control("cancelWorkflowRun"), asked(6)).await;
        assert_eq!(
            error(&response),
            (words.to_owned(), code.to_owned()),
            "{status}"
        );
        assert!(!response.to_string().contains(GITHUB_TEXT), "{response}");
        let entries = audits(&pool).await;
        assert_eq!(entries.len(), n + 1, "one entry for each sent change");
        assert_eq!(entries[n]["change"]["outcome"], outcome, "{status}");
        assert_eq!(entries[n]["change"]["target"], "run#6");
        assert!(!entries[n].to_string().contains(GITHUB_TEXT));
    }
    assert_eq!(
        sent(&forge, "POST", "/actions/runs/6/cancel").len(),
        6,
        "each sent once"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_a_dropped_answer_to_run_again_is_unknown_recorded_and_sent_once(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok(
        "/actions/runs/5",
        &run_in(5, "completed", Some("failure"), 1),
    );
    forge
        .kit
        .stub
        .drop_answer(&format!("{BASE}/actions/runs/5/rerun"));

    let response = forge.ask(&control("rerunWorkflowRun"), asked(5)).await;

    assert_eq!(
        error(&response),
        (UNKNOWN.to_owned(), "UNAVAILABLE".to_owned())
    );
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(
        sent(&forge, "POST", "/actions/runs/5/rerun").len(),
        1,
        "never sent again"
    );
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["change"]["outcome"], "unknown");
    assert_eq!(
        events(&pool).await,
        [(
            "warning".to_owned(),
            "Project widgets: rerun_workflow_run on run#5, sent with no answer from GitHub: \
             it may have been made."
                .to_owned()
        )]
    );
}
