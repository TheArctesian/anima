//! K-E1 Section 13.3, "Pre-read is a fresh GET": the read a change acts on is never a held
//! answer. When GitHub's read fails for a passing reason (a 5xx, a limit, a full queue), the
//! change is refused and sends nothing, where a plain read would serve the held answer stale.

use serde_json::json;
use sqlx::PgPool;

use super::{SET_OPEN, answer, sent};
use crate::{
    hub::error,
    projects::forge::{BASE, forge, issue, reply, run, sha},
};

const UNUSABLE: &str = "GitHub's answer could not be used. Try again.";

#[sqlx::test(migrations = "./migrations")]
async fn prj_034_a_run_control_never_acts_on_a_held_run(pool: PgPool) {
    let forge = forge(&pool).await;
    let mut failed = run(5);
    failed["conclusion"] = json!("failure");
    forge.ok("/actions/runs/5", &failed);
    let one = "query($repository: ID!) { workflowRun(repository: $repository, runId: \"5\") { \
        id status } }";
    let first = forge.ask(one, json!({})).await;
    assert!(first["errors"].is_null(), "{first}");

    // GitHub now fails the read; in life the run may be running again.
    forge.reply("/actions/runs/5", reply(503, &[], "{}"));
    answer(&forge, ("POST", "/actions/runs/5/rerun"), 201, &json!({}));
    forge.clock.advance(600);

    let rerun = "mutation($repository: ID!, $sha: String!) { rerunWorkflowRun(repository: \
        $repository, runId: \"5\", expectedHeadSha: $sha) { id } }";
    let response = forge.ask(rerun, json!({ "sha": sha(5) })).await;

    assert_eq!(
        error(&response),
        (UNUSABLE.to_owned(), "UNAVAILABLE".to_owned()),
        "{response}"
    );
    assert!(sent(&forge, "POST", "/actions/runs/5/rerun").is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_closing_an_issue_never_acts_on_a_held_issue(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/issues/3", &issue(3));
    let one = "query($repository: ID!) { issue(repository: $repository, number: 3) { number } }";
    let first = forge.ask(one, json!({})).await;
    assert!(first["errors"].is_null(), "{first}");

    forge.reply("/issues/3", reply(503, &[], "{}"));
    let path = format!("{BASE}/issues/3");
    (forge.kit.stub).reply_to("PATCH", &path, reply(200, &[], &issue(3).to_string()));
    forge.clock.advance(600);

    let response = forge
        .ask(SET_OPEN, json!({ "number": 3, "open": false }))
        .await;

    assert_eq!(
        error(&response),
        (UNUSABLE.to_owned(), "UNAVAILABLE".to_owned()),
        "{response}"
    );
    assert!(sent(&forge, "PATCH", "/issues/3").is_empty());
}
