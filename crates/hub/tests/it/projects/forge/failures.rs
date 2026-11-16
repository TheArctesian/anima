//! K-E1 Section 13.7, LOG-007: each failure of GitHub reaches the client as its K7 code and
//! fixed words, never as GitHub's own text.

use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{BASE, forge, reply};
use crate::hub::{error, start};

const GITHUB_TEXT: &str = "GITHUB-OWN-TEXT-5150 <script>alert(1)</script>";

fn said() -> String {
    json!({ "message": GITHUB_TEXT }).to_string()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_each_failure_has_its_code_and_fixed_words(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply("/pulls", reply(404, &[], &said()));
    forge.reply("/issues", reply(403, &[], &said()));
    forge.reply(
        "/actions/workflows",
        super::ok(&json!({ "workflows": GITHUB_TEXT }).to_string()),
    );
    forge.kit.stub.endless(&format!("{BASE}/actions/runs"));
    forge.reply("/issues/5/comments", reply(500, &[], &said()));
    forge.reply("/pulls/6", reply(302, &[], &said()));

    let cases = [
        (
            "query($repository: ID!) { pullRequests(repository: $repository) { edges { cursor } } }",
            "GitHub has no such repository, or the App may not read it.",
            "NOT_FOUND",
        ),
        (
            "query($repository: ID!) { issues(repository: $repository) { edges { cursor } } }",
            "The GitHub App is not permitted to do that. Check its permissions on GitHub.",
            "FORBIDDEN",
        ),
        (
            "query($repository: ID!) { workflows(repository: $repository) { id } }",
            "GitHub's answer could not be used. Try again.",
            "UNAVAILABLE",
        ),
        (
            "query($repository: ID!) { workflowRuns(repository: $repository) { edges { cursor } } }",
            "GitHub's answer is larger than the hub reads.",
            "UNAVAILABLE",
        ),
        (
            "query($repository: ID!) { issueComments(repository: $repository, number: 5) { edges { cursor } } }",
            "GitHub's answer could not be used. Try again.",
            "UNAVAILABLE",
        ),
        (
            "query($repository: ID!) { pullRequest(repository: $repository, number: 6) { number } }",
            "GitHub's answer could not be used. Try again.",
            "UNAVAILABLE",
        ),
    ];
    for (query, words, code) in cases {
        let response = forge.ask(query, json!({})).await;
        assert_eq!(
            error(&response),
            (words.to_owned(), code.to_owned()),
            "{query}"
        );
        assert!(
            !response.to_string().contains("GITHUB-OWN-TEXT"),
            "{response}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_rate_limit_says_until_when_and_stops_the_reads(pool: PgPool) {
    let forge = forge(&pool).await;
    let reset = (start() + TimeDelta::minutes(15)).timestamp().to_string();
    forge.reply(
        "/pulls",
        reply(
            403,
            &[
                ("x-ratelimit-remaining", "0"),
                ("x-ratelimit-reset", &reset),
            ],
            &said(),
        ),
    );
    let pulls =
        "query($repository: ID!) { pullRequests(repository: $repository) { edges { cursor } } }";
    let response = forge.ask(pulls, json!({})).await;
    let limited = (
        "GitHub limits requests until 12:15.".to_owned(),
        "UNAVAILABLE".to_owned(),
    );
    assert_eq!(error(&response), limited);

    let issues = "query($repository: ID!) { issues(repository: $repository) { edges { cursor } } }";
    assert_eq!(error(&forge.ask(issues, json!({})).await), limited);
    assert!(
        forge.heard("/issues").is_empty(),
        "nothing is sent until the reset"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_429_lasts_its_retry_after(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply("/issues", reply(429, &[("retry-after", "120")], &said()));
    let issues = "query($repository: ID!) { issues(repository: $repository) { edges { cursor } } }";
    let response = forge.ask(issues, json!({})).await;
    assert_eq!(
        error(&response),
        (
            "GitHub limits requests until 12:02.".to_owned(),
            "UNAVAILABLE".to_owned()
        )
    );
}
