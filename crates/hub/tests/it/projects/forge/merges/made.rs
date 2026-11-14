//! OD-E14, PRJ-028, PRJ-033: a merge leaves exactly as K-E1 Section 13.3 says, bound to the head
//! the operator saw, returns the pull request read back from GitHub, leaves one audit entry and
//! one event, and drops the held pull request answers; one already merged comes back unchanged.

use serde_json::json;
use sqlx::PgPool;

use super::{LIST, clean, merge, merge_sha, merged, methods, open, reads, script, sent};
use crate::projects::forge::{
    changes::{audits, body, events},
    forge, pull, sha,
};

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_clean_merge_puts_the_method_and_the_head_seen_and_returns_it_merged(
    pool: PgPool,
) {
    let forge = forge(&pool).await;
    forge.ok("/pulls", &json!([pull(12, "open")]));
    let listed = forge.ask(LIST, json!({})).await;
    assert!(listed["errors"].is_null(), "{listed}");
    script(&forge, 12);

    let response = merge(&forge, 12, &json!({})).await;

    assert_eq!(
        response["data"]["mergePullRequest"],
        json!({ "repository": forge.repository, "number": 12, "state": "MERGED",
            "headSha": sha(12), "mergedAt": "2030-05-03T10:00:00+00:00", "fromFork": false,
            "mergeability": null }),
        "{response}"
    );
    let puts = sent(&forge, 12);
    assert_eq!(puts.len(), 1);
    assert_eq!(
        body(&puts[0]),
        json!({ "merge_method": "squash", "sha": sha(12) })
    );
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 1, "{entries:?}");
    let entry = &entries[0];
    assert_eq!(entry["action"], "merge");
    assert_eq!(entry["target_type"], "repository");
    assert_eq!(entry["target_id"], forge.repository.as_str());
    assert_eq!(
        (&entry["actor_kind"], &entry["token_kind"]),
        (&json!("user"), &json!("login"))
    );
    assert_eq!(
        entry["change"],
        json!({ "function": "merge_pull_request", "repository": "octo/widgets",
            "target": "pull_request#12", "outcome": "done", "mergeSha": merge_sha(),
            "number": 12, "head": sha(12), "method": "squash" })
    );
    assert_eq!(
        events(&pool).await,
        [(
            "info".to_owned(),
            "Project widgets: merge_pull_request on pull_request#12, done.".to_owned()
        )]
    );
    let again = forge.ask(LIST, json!({})).await;
    assert!(again["errors"].is_null(), "{again}");
    assert_eq!(forge.heard("/pulls").len(), 2, "the held list was dropped");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_the_pull_request_and_the_methods_are_read_fresh_before_the_merge(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    forge.ok("/pulls/12", &clean(12));
    let read = "query($repository: ID!) { pullRequest(repository: $repository, number: 12) { \
        number } pullRequests(repository: $repository) { mergeMethods } }";
    forge.ok("/pulls", &json!([]));
    let response = forge.ask(read, json!({})).await;
    assert!(response["errors"].is_null(), "{response}");
    reads(&forge, 12, &clean(12), &merged(12));
    super::merges(&forge, 12);

    let response = merge(&forge, 12, &json!({ "method": "MERGE" })).await;

    assert!(response["errors"].is_null(), "{response}");
    assert_eq!(
        forge.heard("/pulls/12").len(),
        3,
        "the held one, before, after"
    );
    assert_eq!(forge.heard("").len(), 2, "the methods were read again");
    assert_eq!(body(&sent(&forge, 12)[0])["merge_method"], "merge");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_an_unstable_one_merges_alone_with_allow_failing_checks(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    reads(&forge, 7, &open(7, "unstable", json!(true)), &merged(7));
    super::merges(&forge, 7);

    let response = merge(&forge, 7, &json!({ "allowFailingChecks": true })).await;

    assert_eq!(
        response["data"]["mergePullRequest"]["state"], "MERGED",
        "{response}"
    );
    assert_eq!(sent(&forge, 7).len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_one_already_merged_comes_back_unchanged_and_nothing_is_sent(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    forge.ok("/pulls/12", &merged(12));

    let response = merge(&forge, 12, &json!({ "expectedHeadSha": sha(99) })).await;

    assert_eq!(
        response["data"]["mergePullRequest"]["state"], "MERGED",
        "{response}"
    );
    assert!(sent(&forge, 12).is_empty(), "nothing");
    assert!(audits(&pool).await.is_empty(), "nothing");
}
