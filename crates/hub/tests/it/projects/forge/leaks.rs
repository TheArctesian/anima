//! LOG-007, PRJ-018: text other people wrote on GitHub comes back unchanged as a JSON string,
//! HTML and control characters included, and reaches no process log line; nor does GitHub's
//! text in a failed answer.

use anima_hub::fleet::events::process_log;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{forge, nodes, pull, reply, run};
use crate::logs::Capture;

const MARK: &str = "HOSTILE-7781";

fn hostile(field: &str) -> String {
    format!("{MARK}-{field} <img src=x onerror=alert(1)>\u{0}\u{1b}[31m\u{202e}&amp;\r\n</td>")
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_github_text_comes_back_unchanged_and_is_never_logged(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let forge = forge(&pool).await;
    let mut entry = pull(1, "open");
    entry["title"] = json!(hostile("title"));
    entry["body"] = json!(hostile("body"));
    entry["user"] = json!({ "login": hostile("login") });
    entry["head"]["ref"] = json!(hostile("branch"));
    entry["labels"] = json!([{ "name": hostile("label") }]);
    forge.ok("/pulls", &json!([entry]));
    let mut action = run(9);
    action["actor"] = json!({ "login": hostile("actor") });
    action["head_commit"] = json!({ "message": hostile("commit") });
    forge.ok(
        "/actions/runs",
        &json!({ "total_count": 1, "workflow_runs": [action] }),
    );
    forge.reply(
        "/issues",
        reply(
            422,
            &[],
            &json!({ "message": hostile("error") }).to_string(),
        ),
    );

    let pulls = forge
        .ask(
            "query($repository: ID!) { pullRequests(repository: $repository) { edges { node { \
             title body authorLogin headBranch labels } } } }",
            json!({}),
        )
        .await;
    assert_eq!(
        nodes(&pulls, "pullRequests")[0],
        json!({ "title": hostile("title"), "body": hostile("body"),
            "authorLogin": hostile("login"), "headBranch": hostile("branch"),
            "labels": [hostile("label")] })
    );
    let runs = forge
        .ask(
            "query($repository: ID!) { workflowRuns(repository: $repository) { edges { node { \
             actorLogin commitTitle } } } }",
            json!({}),
        )
        .await;
    let node = &nodes(&runs, "workflowRuns")[0];
    assert_eq!(node["actorLogin"], Value::from(hostile("actor")));
    let first_line = hostile("commit").lines().next().unwrap().to_owned();
    assert_eq!(node["commitTitle"], Value::from(first_line));
    let refused = forge
        .ask(
            "query($repository: ID!) { issues(repository: $repository) { edges { cursor } } }",
            json!({}),
        )
        .await;
    assert!(!refused.to_string().contains(MARK), "{refused}");

    let log = capture.text();
    assert!(!log.is_empty(), "the log was captured");
    assert!(!log.contains(MARK), "GitHub's text is in the log");
}
