//! PRJ-030, PRJ-031: a repository's commits with their checks, its branches and its releases
//! in K1's shape, paged one GitHub page at a time.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{forge, nodes, ok, sha};
use crate::hub::error;

const COMMITS: &str = "query($repository: ID!, $ref: String, $first: Int, $after: String) { \
    commits(repository: $repository, ref: $ref, first: $first, after: $after) { edges { \
    cursor node { repository sha title message authorName authorLogin committedAt parents url } \
    } pageInfo { hasNextPage endCursor } read { stale } } }";

const BRANCHES: &str = "query($repository: ID!, $first: Int, $after: String) { \
    branches(repository: $repository, first: $first, after: $after) { edges { node { name sha \
    protected default } } pageInfo { hasNextPage endCursor } } }";

const RELEASES: &str = "query($repository: ID!) { repositoryReleases(repository: $repository) \
    { edges { node { id tag name draft prerelease authorLogin createdAt publishedAt url } } \
    pageInfo { hasNextPage } read { stale } } }";

/// A commit as GitHub lists it, numbered `n`, with one parent.
fn commit(n: u64) -> Value {
    json!({
        "sha": sha(n), "html_url": format!("https://github.com/octo/widgets/commit/{}", sha(n)),
        "commit": { "message": format!("Commit {n}"),
            "author": { "name": "Ada", "date": "2030-05-01T10:00:00Z" }, "committer": null },
        "author": { "login": "octo" }, "parents": [{ "sha": sha(n + 1) }]
    })
}

fn shas(nodes: &[Value]) -> Vec<String> {
    nodes
        .iter()
        .map(|node| node["sha"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_commits_come_back_in_k1_shape(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/commits",
        ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_commits_list.json"
        )),
    );

    let response = forge.ask(COMMITS, json!({ "ref": "feature/a#b?c" })).await;
    let commits = nodes(&response, "commits");
    assert_eq!(commits.len(), 3, "{response}");
    let url = "https://github.com/sample-org/sample-repo/commit/";
    assert_eq!(
        commits[0],
        json!({
            "repository": forge.repository, "sha": "1".repeat(40), "title": "Add the widget",
            "message": "It draws a box.\nSecond body line.", "authorName": "Ada Sample",
            "authorLogin": "octo-writer", "committedAt": "2026-11-02T09:30:00+00:00",
            "parents": ["c".repeat(40)], "url": format!("{url}{}", "1".repeat(40))
        })
    );
    assert_eq!(commits[1]["message"], Value::Null);
    assert_eq!(commits[1]["authorLogin"], Value::Null);
    assert_eq!(
        commits[1]["parents"],
        json!(["c".repeat(40), "c".repeat(40)])
    );
    assert_eq!(commits[2]["parents"], json!([]));
    assert_eq!(commits[2]["committedAt"], "2026-11-02T09:00:00+00:00");
    assert_eq!(
        forge.heard("/commits"),
        ["/repos/octo/widgets/commits?sha=feature%2Fa%23b%3Fc&per_page=100&page=1"],
        "the ref goes only in the query, encoded"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_a_long_title_and_message_are_cut_as_k1_states(pool: PgPool) {
    let forge = forge(&pool).await;
    let mut long = commit(1);
    let message = format!("{}\n\n{}", "t".repeat(250), "m".repeat(6_000));
    long["commit"]["message"] = json!(message);
    forge.ok("/commits", &json!([long]));
    let response = forge.ask(COMMITS, json!({})).await;
    let node = &nodes(&response, "commits")[0];
    assert_eq!(node["title"], "t".repeat(200));
    assert_eq!(node["message"], "m".repeat(5_000));
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_each_commit_brings_its_checks_when_selected(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/commits", &json!([commit(1), commit(2)]));
    for n in [1, 2] {
        let check = json!({ "name": "build", "status": "completed", "conclusion": "success" });
        forge.ok(
            &format!("/commits/{}/check-runs", sha(n)),
            &json!({ "total_count": 1, "check_runs": [check] }),
        );
        forge.ok(
            &format!("/commits/{}/status", sha(n)),
            &json!({ "state": "pending", "total_count": 0, "statuses": [] }),
        );
    }
    let query = "query($repository: ID!) { commits(repository: $repository, first: 1) { edges \
        { node { sha checks { sha state passed failed pending other failing partial } } } } }";
    let response = forge.ask(query, json!({})).await;
    assert_eq!(
        nodes(&response, "commits"),
        [
            json!({ "sha": sha(1), "checks": { "sha": sha(1), "state": "PASSED", "passed": 1,
            "failed": 0, "pending": 0, "other": 0, "failing": [], "partial": false } })
        ]
    );
    assert!(
        forge
            .heard(&format!("/commits/{}/check-runs", sha(2)))
            .is_empty(),
        "only the rows shown are checked"
    );
    assert_eq!(forge.heard(&format!("/commits/{}/status", sha(1))).len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_commits_page_forward_one_github_page_at_a_time(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/commits", &Value::Array((1..=100).map(commit).collect()));

    let first = forge.ask(COMMITS, json!({ "first": 30 })).await;
    assert_eq!(
        shas(&nodes(&first, "commits")),
        (1..=30).map(sha).collect::<Vec<_>>()
    );
    let end = &first["data"]["commits"]["pageInfo"];
    assert_eq!(end["hasNextPage"], true);

    let second = forge
        .ask(COMMITS, json!({ "first": 30, "after": end["endCursor"] }))
        .await;
    assert_eq!(
        shas(&nodes(&second, "commits")),
        (31..=60).map(sha).collect::<Vec<_>>()
    );
    assert_eq!(forge.heard("/commits").len(), 1, "one GitHub page, kept");

    let other_size = forge
        .ask(COMMITS, json!({ "first": 20, "after": end["endCursor"] }))
        .await;
    assert_eq!(error(&other_size).1, "VALIDATION");
    for bad in [0, 101] {
        let response = forge.ask(COMMITS, json!({ "first": bad })).await;
        assert_eq!(error(&response).1, "VALIDATION", "{bad}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_a_ref_github_does_not_have_is_not_found(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply("/commits", super::reply(404, &[], "{}"));
    let response = forge.ask(COMMITS, json!({ "ref": "gone" })).await;
    assert_eq!(
        error(&response),
        (
            "GitHub has no such branch or commit, or the App may not read it.".to_owned(),
            "NOT_FOUND".to_owned()
        )
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_branches_come_back_in_k1_shape_with_the_default_marked(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/branches",
        ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_branches_list.json"
        )),
    );
    let first = forge.ask(BRANCHES, json!({ "first": 2 })).await;
    assert_eq!(
        nodes(&first, "branches"),
        [
            json!({ "name": "main", "sha": "4".repeat(40), "protected": true, "default": true }),
            json!({ "name": "feature/widget", "sha": "5".repeat(40), "protected": false,
                "default": false }),
        ]
    );
    let end = &first["data"]["branches"]["pageInfo"];
    assert_eq!(end["hasNextPage"], true, "the rest of GitHub's page");
    let rest = forge
        .ask(BRANCHES, json!({ "first": 2, "after": end["endCursor"] }))
        .await;
    assert_eq!(
        nodes(&rest, "branches"),
        [json!({ "name": "old", "sha": "6".repeat(40), "protected": false, "default": false })]
    );
    assert_eq!(rest["data"]["branches"]["pageInfo"]["hasNextPage"], false);
    assert_eq!(forge.heard("/branches").len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_031_releases_come_back_in_k1_shape(pool: PgPool) {
    let forge = forge(&pool).await;
    let mut releases: Value = serde_json::from_str(include_str!(
        "../../../../../providers/github/tests/fixtures/live_releases_list.json"
    ))
    .unwrap();
    releases[0]["created_at"] = json!("2026-10-29T12:00:00Z");
    releases[1]["created_at"] = json!("2026-11-01T08:00:00Z");
    forge.ok("/releases", &releases);

    let response = forge.ask(RELEASES, json!({})).await;
    let url = "https://github.com/sample-org/sample-repo/releases/tag/";
    assert_eq!(
        nodes(&response, "repositoryReleases"),
        [
            json!({ "id": "801", "tag": "v1.2.0", "name": "Version 1.2", "draft": false,
                "prerelease": false, "authorLogin": "octo-writer",
                "createdAt": "2026-10-29T12:00:00+00:00",
                "publishedAt": "2026-10-30T12:00:00+00:00", "url": format!("{url}v1.2.0") }),
            json!({ "id": "802", "tag": "v1.3.0-rc1", "name": null, "draft": true,
                "prerelease": true, "authorLogin": "ghost",
                "createdAt": "2026-11-01T08:00:00+00:00", "publishedAt": null,
                "url": format!("{url}untagged-abc") }),
        ]
    );
    assert_eq!(
        response["data"]["repositoryReleases"]["pageInfo"]["hasNextPage"],
        false
    );
}
