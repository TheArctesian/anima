//! K-E1 Section 13.5, D-23: every live read root answers a login token, a read key and a write
//! key; signed out, each is `UNAUTHENTICATED`.

use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{Forge, forge, issue, pull, run};
use crate::hub::{error, start, unauthenticated};

const ROOTS: [(&str, &str); 8] = [
    (
        "pullRequests",
        "query($repository: ID!) { pullRequests(repository: $repository) { edges { node { number } } } }",
    ),
    (
        "pullRequest",
        "query($repository: ID!) { pullRequest(repository: $repository, number: 1) { number } }",
    ),
    (
        "issues",
        "query($repository: ID!) { issues(repository: $repository) { edges { node { number } } } }",
    ),
    (
        "issue",
        "query($repository: ID!) { issue(repository: $repository, number: 2) { number } }",
    ),
    (
        "issueComments",
        "query($repository: ID!) { issueComments(repository: $repository, number: 2) { edges { cursor } } }",
    ),
    (
        "workflows",
        "query($repository: ID!) { workflows(repository: $repository) { id } }",
    ),
    (
        "workflowRuns",
        "query($repository: ID!) { workflowRuns(repository: $repository) { edges { node { id } } } }",
    ),
    (
        "workflowRun",
        "query($repository: ID!) { workflowRun(repository: $repository, runId: \"3\") { id } }",
    ),
];

/// The secret of a key of `scope` made by the signed-in operator.
async fn key(forge: &Forge, scope: &str) -> String {
    let query = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let input = json!({ "input": { "name": scope, "scope": scope, "madeFor": "USER",
        "expiresAt": start() + TimeDelta::days(30) } });
    let response = forge.kit.post(query, input).await;
    response["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("no key: {response}"))
        .to_owned()
}

fn script(forge: &Forge) {
    forge.ok("/pulls", &json!([pull(1, "open")]));
    forge.ok("/pulls/1", &pull(1, "open"));
    forge.ok("/issues", &json!([issue(2)]));
    forge.ok("/issues/2", &issue(2));
    forge.ok("/issues/2/comments", &json!([]));
    forge.ok(
        "/actions/workflows",
        &json!({ "total_count": 0, "workflows": [] }),
    );
    forge.ok(
        "/actions/runs",
        &json!({ "total_count": 1, "workflow_runs": [run(3)] }),
    );
    forge.ok("/actions/runs/3", &run(3));
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_every_live_read_answers_a_login_token_and_both_key_scopes(pool: PgPool) {
    let forge = forge(&pool).await;
    script(&forge);
    let tokens = [
        forge.kit.token.clone(),
        key(&forge, "READ").await,
        key(&forge, "WRITE").await,
    ];
    let variables = json!({ "repository": forge.repository });
    for (root, query) in ROOTS {
        for token in &tokens {
            let response = forge
                .kit
                .hub
                .post(Some(token), query, variables.clone())
                .await;
            assert!(response["errors"].is_null(), "{root}: {response}");
            assert_ne!(response["data"][root], Value::Null, "{root}: {response}");
        }
        let signed_out = forge.kit.hub.post(None, query, variables.clone()).await;
        assert_eq!(error(&signed_out), unauthenticated(), "{root}");
    }
}
