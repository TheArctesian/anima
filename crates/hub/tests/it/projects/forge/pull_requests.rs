//! PRJ-031, PRJ-033: a repository's pull requests in K1's shape, paged forward to the end, with
//! each mergeability state, the checks of the head commit and the merge methods.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{forge, nodes, numbers, pull, sha};
use crate::hub::start;

const LIST: &str = "query($repository: ID!, $first: Int, $after: String, $state: OpenOrClosed) { \
    pullRequests(repository: $repository, first: $first, after: $after, state: $state) { \
    edges { cursor node { repository number title state draft authorLogin headBranch headSha \
    baseBranch fromFork headRepository labels comments createdAt updatedAt mergedAt url body \
    bodyCut } } pageInfo { hasNextPage endCursor } read { at stale problem } } }";

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_the_list_comes_back_in_k1_shape(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/pulls",
        super::ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_pulls_list.json"
        )),
    );
    let mut one = pull(12, "open");
    one["comments"] = json!(4);
    forge.ok("/pulls/12", &one);

    let response = forge.ask(LIST, json!({ "state": "ALL" })).await;
    let pulls = nodes(&response, "pullRequests");
    assert_eq!(numbers(&pulls), [12, 9, 7], "{response}");
    assert_eq!(
        pulls[0],
        json!({
            "repository": forge.repository, "number": 12, "title": "Add the sample widget",
            "state": "OPEN", "draft": false, "authorLogin": "octo-writer",
            "headBranch": "feature/widget", "headSha": "1".repeat(40), "baseBranch": "main",
            "fromFork": false, "headRepository": "sample-org/sample-repo",
            "labels": ["enhancement", "ui"], "comments": 4,
            "createdAt": "2026-11-01T09:00:00+00:00", "updatedAt": "2026-11-02T10:30:00+00:00",
            "mergedAt": null, "url": "https://github.com/sample-org/sample-repo/pull/12",
            "body": "Adds the widget.\n\n<b>not html here</b>", "bodyCut": false
        })
    );
    let (merged, orphan) = (&pulls[1], &pulls[2]);
    assert_eq!(
        (&merged["state"], &merged["fromFork"]),
        (&json!("MERGED"), &json!(true))
    );
    assert_eq!(merged["authorLogin"], "ghost", "a deleted account");
    assert_eq!(
        (&orphan["state"], &orphan["headRepository"]),
        (&json!("CLOSED"), &Value::Null)
    );
    let read = &response["data"]["pullRequests"]["read"];
    assert_eq!(
        *read,
        json!({ "at": start().to_rfc3339(), "stale": false, "problem": null })
    );
    assert!(forge.heard("/pulls")[0].contains("state=all"));
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_pages_go_forward_to_the_end_one_github_page_at_a_time(pool: PgPool) {
    let forge = forge(&pool).await;
    let page =
        |from: u64, count: u64| Value::Array((0..count).map(|n| pull(from - n, "open")).collect());
    forge.ok("/pulls", &page(300, 100));

    let mut seen = Vec::new();
    let mut after = Value::Null;
    let mut pages = 0;
    loop {
        let response = forge
            .ask(LIST, json!({ "first": 50, "after": after }))
            .await;
        seen.extend(numbers(&nodes(&response, "pullRequests")));
        pages += 1;
        let end = &response["data"]["pullRequests"]["pageInfo"];
        if end["hasNextPage"] == false {
            break;
        }
        after = end["endCursor"].clone();
        if pages == 2 {
            assert_eq!(
                forge.heard("/pulls").len(),
                1,
                "both halves of one GitHub page"
            );
            forge.ok("/pulls", &page(200, 20));
        }
    }
    assert_eq!(pages, 3);
    assert_eq!(seen, (181..=300).rev().collect::<Vec<_>>());
    let heard = forge.heard("/pulls");
    assert_eq!(heard.len(), 2, "{heard:?}");
    assert!(heard[1].contains("page=2"), "{heard:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_a_bad_first_or_cursor_is_validation(pool: PgPool) {
    let forge = forge(&pool).await;
    for variables in [
        json!({ "first": 51 }),
        json!({ "first": 0 }),
        json!({ "after": "g1.1.0.30" }),
        json!({ "after": "not a cursor" }),
    ] {
        let response = forge.ask(LIST, variables.clone()).await;
        assert_eq!(
            response["errors"][0]["extensions"]["code"], "VALIDATION",
            "{variables}"
        );
    }
    assert_eq!(forge.live_reads(), 0, "nothing was sent");
}

const MERGEABILITY: &str = "query($repository: ID!) { pullRequests(repository: $repository) { \
    edges { node { number mergeability { state canMerge } } } } }";

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_each_mergeable_state_maps_to_one_state_and_only_clean_can_merge(pool: PgPool) {
    let forge = forge(&pool).await;
    let cases: [(Value, Value, bool, &str, bool); 11] = [
        (json!("clean"), json!(true), false, "CLEAN", true),
        (json!("has_hooks"), json!(true), false, "CLEAN", true),
        (json!("clean"), json!(true), true, "CLEAN", false),
        (json!("clean"), Value::Null, false, "CLEAN", false),
        (json!("unstable"), json!(true), false, "UNSTABLE", false),
        (json!("dirty"), json!(false), false, "CONFLICT", false),
        (json!("blocked"), json!(true), false, "BLOCKED", false),
        (json!("behind"), json!(true), false, "BEHIND", false),
        (json!("draft"), json!(true), true, "DRAFT", false),
        (json!("unknown"), Value::Null, false, "UNKNOWN", false),
        (json!("made_up_later"), json!(true), false, "UNKNOWN", false),
    ];
    let mut listed: Vec<Value> = Vec::new();
    for (n, (state, mergeable, draft, _, _)) in (1_u64..).zip(&cases) {
        listed.push(pull(n, "open"));
        let mut one = pull(n, "open");
        one["mergeable_state"] = state.clone();
        one["mergeable"] = mergeable.clone();
        one["draft"] = json!(draft);
        forge.ok(&format!("/pulls/{n}"), &one);
    }
    listed.push(pull(99, "closed"));
    forge.ok("/pulls", &Value::Array(listed));

    let response = forge.ask(MERGEABILITY, json!({})).await;
    let pulls = nodes(&response, "pullRequests");
    for (node, (state, _, _, expected, can_merge)) in pulls.iter().zip(&cases) {
        let expected = json!({ "state": expected, "canMerge": can_merge });
        assert_eq!(node["mergeability"], expected, "{state}");
    }
    assert_eq!(
        pulls[11]["mergeability"],
        Value::Null,
        "a closed one is not read"
    );
    assert_eq!(forge.heard("/pulls/99"), Vec::<String>::new());
    assert_eq!(forge.heard("/pulls/1").len(), 1, "one read each");

    let one = "query($repository: ID!, $number: Int!) { pullRequest(repository: $repository, \
        number: $number) { number mergeability { state canMerge } } }";
    let response = forge.ask(one, json!({ "number": 1 })).await;
    assert_eq!(
        response["data"]["pullRequest"]["mergeability"]["canMerge"],
        true
    );
    assert_eq!(forge.heard("/pulls/1").len(), 1, "the single read was held");
    let missing = forge.ask(one, json!({ "number": 4040 })).await;
    assert_eq!(missing, json!({ "data": { "pullRequest": null } }));
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_checks_count_runs_and_statuses_of_the_head_commit(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/pulls", &json!([pull(1, "open"), pull(2, "open")]));
    let checks = |n: u64| format!("/commits/{}/check-runs", sha(n));
    let status = |n: u64| format!("/commits/{}/status", sha(n));
    let check = |name: &str, status: &str, conclusion: Value| json!({ "name": name, "status": status, "conclusion": conclusion });
    forge.ok(
        &checks(1),
        &json!({ "total_count": 4, "check_runs": [
            check("build", "completed", json!("success")),
            check("<b>lint</b>", "completed", json!("failure")),
            check("e2e", "in_progress", Value::Null),
            check("docs", "completed", json!("skipped")),
        ] }),
    );
    forge.ok(
        &status(1),
        &json!({ "state": "success", "total_count": 1,
            "statuses": [{ "context": "ci/legacy", "state": "success" }] }),
    );
    forge.ok(&checks(2), &json!({ "total_count": 0, "check_runs": [] }));
    forge.ok(
        &status(2),
        &json!({ "state": "pending", "total_count": 0, "statuses": [] }),
    );

    let query = "query($repository: ID!) { pullRequests(repository: $repository) { edges { node { \
        checks { sha state passed failed pending other failing partial } } } } }";
    let response = forge.ask(query, json!({})).await;
    let pulls = nodes(&response, "pullRequests");
    assert_eq!(
        pulls[0]["checks"],
        json!({ "sha": sha(1), "state": "FAILED", "passed": 2, "failed": 1, "pending": 1,
            "other": 1, "failing": ["<b>lint</b>"], "partial": false })
    );
    assert_eq!(
        pulls[1]["checks"],
        json!({ "sha": sha(2), "state": "NONE", "passed": 0, "failed": 0, "pending": 0,
            "other": 0, "failing": [], "partial": false }),
        "a combined status with no statuses is not pending"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_merge_methods_are_those_the_repository_allows(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/pulls", &json!([]));
    let query =
        "query($repository: ID!) { pullRequests(repository: $repository) { mergeMethods } }";
    forge.reply(
        "",
        super::ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_repository_merge_methods.json"
        )),
    );
    let response = forge.ask(query, json!({})).await;
    assert_eq!(
        response["data"]["pullRequests"]["mergeMethods"],
        json!(["SQUASH", "REBASE"])
    );

    forge.clock.advance(601);
    forge.ok("", &json!({ "id": 1, "full_name": super::NAME }));
    let response = forge.ask(query, json!({})).await;
    let methods = &response["data"]["pullRequests"]["mergeMethods"];
    assert_eq!(
        *methods,
        json!([]),
        "without Contents write, none is guessed"
    );
}
