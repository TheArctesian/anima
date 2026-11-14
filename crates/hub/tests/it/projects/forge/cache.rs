//! PRJ-032, K-E1 Section 13.1: a second read inside the window sends nothing; `fresh` sends
//! once the held answer is five seconds old; a failed refresh serves the held answer stale.

use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{forge, issue, nodes, numbers, pull, reply};
use crate::{hub::start, sources::set_projects};

const PULLS: &str = "query($repository: ID!, $fresh: Boolean) { pullRequests(repository: \
    $repository, fresh: $fresh) { edges { node { number } } read { at stale problem } } }";

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_second_read_inside_the_window_sends_nothing(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/pulls", &json!([pull(1, "open")]));

    forge.ask(PULLS, json!({})).await;
    forge.clock.advance(29);
    let second = forge.ask(PULLS, json!({})).await;
    assert_eq!(forge.heard("/pulls").len(), 1);
    assert_eq!(
        second["data"]["pullRequests"]["read"]["at"],
        start().to_rfc3339()
    );

    forge.clock.advance(2);
    forge.ask(PULLS, json!({})).await;
    assert_eq!(forge.heard("/pulls").len(), 2, "30 seconds: read again");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_fresh_sends_once_the_held_answer_is_five_seconds_old(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/pulls", &json!([pull(1, "open")]));
    forge.ask(PULLS, json!({})).await;

    forge.clock.advance(4);
    forge.ask(PULLS, json!({ "fresh": true })).await;
    assert_eq!(forge.heard("/pulls").len(), 1, "under five seconds old");

    forge.ok("/pulls", &json!([pull(2, "open"), pull(1, "open")]));
    forge.clock.advance(1);
    let fresh = forge.ask(PULLS, json!({ "fresh": true })).await;
    assert_eq!(forge.heard("/pulls").len(), 2);
    assert_eq!(numbers(&nodes(&fresh, "pullRequests")), [2, 1]);
    let at = start() + TimeDelta::seconds(5);
    assert_eq!(fresh["data"]["pullRequests"]["read"]["at"], at.to_rfc3339());
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_failed_refresh_serves_the_held_answer_stale(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/pulls", &json!([pull(1, "open")]));
    forge.ask(PULLS, json!({})).await;

    forge.reply(
        "/pulls",
        reply(502, &[], "{\"message\":\"GITHUB-TEXT-502\"}"),
    );
    forge.clock.advance(60);
    let stale = forge.ask(PULLS, json!({})).await;
    assert_eq!(numbers(&nodes(&stale, "pullRequests")), [1]);
    assert_eq!(
        stale["data"]["pullRequests"]["read"],
        json!({ "at": start().to_rfc3339(), "stale": true,
            "problem": "GitHub's answer could not be used. Try again." })
    );
    assert!(!stale.to_string().contains("GITHUB-TEXT"));
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_002_every_live_root_is_conflict_while_projects_is_off(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/issues", &json!([issue(1)]));
    set_projects(&pool, false).await;
    let roots = [
        "query($repository: ID!) { pullRequests(repository: $repository) { edges { cursor } } }",
        "query($repository: ID!) { pullRequest(repository: $repository, number: 1) { number } }",
        "query($repository: ID!) { issues(repository: $repository) { edges { cursor } } }",
        "query($repository: ID!) { issue(repository: $repository, number: 1) { number } }",
        "query($repository: ID!) { issueComments(repository: $repository, number: 1) { edges { cursor } } }",
        "query($repository: ID!) { workflows(repository: $repository) { id } }",
        "query($repository: ID!) { workflowRuns(repository: $repository) { edges { cursor } } }",
        "query($repository: ID!) { workflowRun(repository: $repository, runId: \"1\") { id } }",
    ];
    for query in roots {
        let response = forge.ask(query, json!({})).await;
        assert_eq!(
            response["errors"][0]["extensions"]["code"], "CONFLICT",
            "{query}"
        );
        let message = response["errors"][0]["message"].as_str().unwrap();
        assert!(message.contains("Projects is off"), "{message}");
    }
    assert_eq!(forge.live_reads(), 0);
}
