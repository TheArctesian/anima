//! K-E1 Section 13.1, "Request budget": one GraphQL request asks GitHub at most 60 times for an
//! API key and 200 times for a login or browser token. The reads past the budget are refused
//! with fixed words, and a normal page with its checks stays well inside it.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{Forge, forge, merges::clean, nodes, pull, sha, tokens::key};

const SPENT: &str = "This request asked GitHub too many times. Ask for less at once.";

/// Two aliased lists of 50 pull requests, each with its mergeability and checks.
const TWICE: &str = "query($repository: ID!) { \
    a: pullRequests(repository: $repository, first: 50) { edges { node { number \
      mergeability { state } checks { state } } } } \
    b: pullRequests(repository: $repository, first: 50, state: ALL) { edges { node { number \
      mergeability { state } checks { state } } } } }";

/// GitHub lists `count` open, clean pull requests, each with one passed check and no status.
fn listed(forge: &Forge, count: u64) {
    let pulls: Vec<Value> = (1..=count).map(|n| pull(n, "open")).collect();
    forge.ok("/pulls", &Value::Array(pulls));
    for n in 1..=count {
        forge.ok(&format!("/pulls/{n}"), &clean(n));
        let passed = json!({ "name": "build", "status": "completed", "conclusion": "success" });
        forge.ok(
            &format!("/commits/{}/check-runs", sha(n)),
            &json!({ "total_count": 1, "check_runs": [passed] }),
        );
        forge.ok(
            &format!("/commits/{}/status", sha(n)),
            &json!({ "state": "pending", "total_count": 0, "statuses": [] }),
        );
    }
}

/// The messages of the errors of `response`.
fn messages(response: &Value) -> Vec<String> {
    let errors = response["errors"].as_array().map_or(&[][..], Vec::as_slice);
    errors
        .iter()
        .map(|e| e["message"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_one_api_key_request_asks_github_at_most_60_times(pool: PgPool) {
    let forge = forge(&pool).await;
    listed(&forge, 50);
    let read_key = key(&forge, "READ").await;
    let before = forge.live_reads();

    let response = forge
        .kit
        .hub
        .post(
            Some(&read_key),
            TWICE,
            json!({ "repository": forge.repository }),
        )
        .await;

    let spent = forge.live_reads() - before;
    assert!(
        spent <= 60,
        "one request of a key cost {spent} GitHub requests"
    );
    let refused = messages(&response);
    assert!(!refused.is_empty(), "{response}");
    assert!(refused.iter().all(|m| m == SPENT), "{refused:?}");
    let codes = &response["errors"][0]["extensions"]["code"];
    assert_eq!(codes, "UNAVAILABLE");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_one_login_request_asks_github_at_most_200_times(pool: PgPool) {
    let forge = forge(&pool).await;
    listed(&forge, 50);
    let before = forge.live_reads();

    let response = forge.ask(TWICE, json!({})).await;

    let spent = forge.live_reads() - before;
    assert!(
        spent <= 200,
        "one login request cost {spent} GitHub requests"
    );
    assert!(messages(&response).iter().all(|m| m == SPENT), "{response}");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_page_of_30_pull_requests_with_checks_fits_a_login_request(pool: PgPool) {
    let forge = forge(&pool).await;
    listed(&forge, 30);
    let query = "query($repository: ID!) { pullRequests(repository: $repository, first: 30) { \
        edges { node { number mergeability { state } checks { state passed } } } } }";

    let response = forge.ask(query, json!({})).await;

    assert!(response["errors"].is_null(), "{response}");
    let pulls = nodes(&response, "pullRequests");
    assert_eq!(pulls.len(), 30);
    assert!(
        pulls
            .iter()
            .all(|p| p["checks"]["state"] == "PASSED" && p["checks"]["passed"] == 1),
        "{response}"
    );
}
