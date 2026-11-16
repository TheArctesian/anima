//! PRJ-031: a repository's issues in K1's shape with no pull request among them, one issue,
//! and the comments of an issue or a pull request.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{forge, issue, nodes, numbers, pull_as_issue, reply};
use crate::hub::error;

const LIST: &str = "query($repository: ID!, $state: OpenOrClosed, $after: String) { \
    issues(repository: $repository, state: $state, after: $after) { edges { cursor node { \
    repository number title open closeReason authorLogin labels assignees comments createdAt \
    updatedAt closedAt url body bodyCut } } pageInfo { hasNextPage endCursor } \
    read { at stale problem } } }";

const ONE: &str = "query($repository: ID!, $number: Int!) { issue(repository: $repository, \
    number: $number) { number title open } }";

const COMMENTS: &str = "query($repository: ID!, $number: Int!) { issueComments(repository: \
    $repository, number: $number) { edges { node { id authorLogin body bodyCut createdAt \
    updatedAt url } } pageInfo { hasNextPage } read { stale } } }";

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_the_issue_list_has_no_pull_requests(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/issues",
        super::ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_issues_list.json"
        )),
    );

    let response = forge.ask(LIST, json!({ "state": "ALL" })).await;
    let issues = nodes(&response, "issues");
    assert_eq!(numbers(&issues), [21, 5, 4], "pull request 12 is left out");
    assert_eq!(
        issues[0],
        json!({
            "repository": forge.repository, "number": 21, "title": "The sample page is slow",
            "open": true, "closeReason": null, "authorLogin": "octo-reporter",
            "labels": ["bug", "perf"], "assignees": ["octo-writer"], "comments": 2,
            "createdAt": issues[0]["createdAt"], "updatedAt": issues[0]["updatedAt"],
            "closedAt": null, "url": "https://github.com/sample-org/sample-repo/issues/21",
            "body": "It takes ten seconds.", "bodyCut": false
        })
    );
    assert_eq!(issues[1]["closeReason"], "NOT_PLANNED");
    assert_eq!(
        issues[2]["closeReason"],
        Value::Null,
        "a reason K1 does not name"
    );
    assert_eq!(response["data"]["issues"]["pageInfo"]["hasNextPage"], false);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_a_page_of_only_pull_requests_still_has_a_next_page(pool: PgPool) {
    let forge = forge(&pool).await;
    let pulls: Vec<Value> = (1..=100).map(pull_as_issue).collect();
    forge.ok("/issues", &Value::Array(pulls));

    let response = forge.ask(LIST, json!({})).await;
    assert_eq!(nodes(&response, "issues"), Vec::<Value>::new());
    let end = &response["data"]["issues"]["pageInfo"];
    assert_eq!(end["hasNextPage"], true, "{response}");

    forge.ok("/issues", &json!([issue(500), pull_as_issue(501)]));
    let next = forge.ask(LIST, json!({ "after": end["endCursor"] })).await;
    assert_eq!(numbers(&nodes(&next, "issues")), [500]);
    assert_eq!(next["data"]["issues"]["pageInfo"]["hasNextPage"], false);
    assert!(forge.heard("/issues")[1].contains("page=2"));
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_one_issue_is_null_when_missing_or_a_pull_request(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/issues/3", &issue(3));
    forge.ok("/issues/12", &pull_as_issue(12));

    let found = forge.ask(ONE, json!({ "number": 3 })).await;
    assert_eq!(
        found["data"]["issue"],
        json!({ "number": 3, "title": "Issue 3", "open": true })
    );
    for number in [12, 404] {
        let response = forge.ask(ONE, json!({ "number": number })).await;
        assert_eq!(response, json!({ "data": { "issue": null } }), "{number}");
    }
    let refused = forge.ask(ONE, json!({ "number": 0 })).await;
    assert_eq!(refused["errors"][0]["extensions"]["code"], "VALIDATION");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_comments_come_oldest_first_in_k1_shape(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/issues/21/comments",
        super::ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_issue_comments.json"
        )),
    );

    let response = forge.ask(COMMENTS, json!({ "number": 21 })).await;
    let comments = nodes(&response, "issueComments");
    assert_eq!(
        comments[0],
        json!({ "id": "9001", "authorLogin": "octo-writer", "body": "First reply.",
            "bodyCut": false, "createdAt": "2026-11-03T10:00:00+00:00",
            "updatedAt": "2026-11-03T10:00:00+00:00",
            "url": "https://github.com/sample-org/sample-repo/issues/21#issuecomment-9001" })
    );
    assert_eq!(
        (&comments[1]["authorLogin"], &comments[1]["body"]),
        (&json!("ghost"), &json!(""))
    );

    let missing = forge.ask(COMMENTS, json!({ "number": 77 })).await;
    assert_eq!(
        error(&missing),
        (
            "GitHub has no such issue or pull request, or the App may not read it.".to_owned(),
            "NOT_FOUND".to_owned()
        )
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_a_body_is_cut_at_20000_characters_and_marked(pool: PgPool) {
    let forge = forge(&pool).await;
    let mut long = issue(8);
    long["body"] = json!("é".repeat(20_005));
    long["title"] = json!("t".repeat(400));
    long["labels"] = Value::Array(
        (0..30)
            .map(|n| json!({ "name": format!("l{n}") }))
            .collect(),
    );
    forge.ok("/issues", &json!([long]));

    let response = forge.ask(LIST, json!({})).await;
    let node = &nodes(&response, "issues")[0];
    assert_eq!(node["body"].as_str().unwrap().chars().count(), 20_000);
    assert_eq!(node["bodyCut"], true);
    assert_eq!(node["title"].as_str().unwrap().len(), 300);
    assert_eq!(node["labels"].as_array().unwrap().len(), 20);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_issues_off_is_conflict(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/issues",
        reply(410, &[], "{\"message\":\"Issues are disabled\"}"),
    );
    let response = forge.ask(LIST, json!({})).await;
    assert_eq!(
        error(&response),
        (
            "Issues are off for this repository.".to_owned(),
            "CONFLICT".to_owned()
        )
    );
}
