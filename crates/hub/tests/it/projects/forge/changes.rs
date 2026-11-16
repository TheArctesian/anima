//! The first changes Anima makes on GitHub, through GraphQL against the loopback GitHub
//! (OD-E14, PRJ-027, PRJ-028; K-E1 Sections 13.2 to 13.7): make an issue, comment, close or
//! open one; run a workflow run again, run its failed jobs again, cancel it, start a workflow
//! and write a commit status (PRJ-034, PRJ-035, PRO-010). What the stub hears, the K1 result, the record each sent change leaves, the held
//! answers it drops, the fixed words of every outcome, the refusals that send nothing, the token
//! kinds, and the text that never reaches a log, an error or a record.

mod controls;
mod fences;
mod fresh;
mod guarded;
mod made;
mod outcomes;
mod refusals;
mod secrets;
mod starts;
mod tokens;

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{BASE, Forge, issue, reply};
use crate::sources::stub::Heard;

pub const CREATE: &str = "mutation($repository: ID!, $title: String!, $body: String, \
    $labels: [String!]) { createIssue(input: { repository: $repository, title: $title, \
    body: $body, labels: $labels }) { repository number title open closeReason authorLogin \
    labels assignees comments createdAt updatedAt closedAt url body bodyCut } }";

pub const COMMENT: &str = "mutation($repository: ID!, $number: Int!, $body: String!) { \
    addIssueComment(repository: $repository, number: $number, body: $body) { id authorLogin \
    body bodyCut createdAt updatedAt url } }";

pub const SET_OPEN: &str = "mutation($repository: ID!, $number: Int!, $open: Boolean!, \
    $reason: IssueCloseReason) { setIssueOpen(repository: $repository, number: $number, \
    open: $open, reason: $reason) { number open closeReason closedAt } }";

pub const LIST: &str =
    "query($repository: ID!) { issues(repository: $repository) { edges { cursor } } }";

/// The requests with `method` for `BASE` + `tail` so far, in full.
pub fn sent(forge: &Forge, method: &str, tail: &str) -> Vec<Heard> {
    let path = format!("{BASE}{tail}");
    forge.kit.stub.with(|s| {
        s.heard
            .iter()
            .filter(|h| h.method == method && h.path == path)
            .cloned()
            .collect()
    })
}

/// The JSON body of a heard request.
pub fn body(heard: &Heard) -> Value {
    serde_json::from_slice(&heard.body).unwrap()
}

/// GitHub answers each `method` request for `BASE` + `tail` with `status` and `body`.
pub fn answer(forge: &Forge, (method, tail): (&str, &str), status: u16, body: &Value) {
    let path = format!("{BASE}{tail}");
    (forge.kit.stub).reply_to(method, &path, reply(status, &[], &body.to_string()));
}

/// The audit entries of changes to repositories, oldest first.
pub async fn audits(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM events.audit_log t WHERE target_type = 'repository' \
         ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// The `(severity, message)` of the events of changes in project `widgets`.
pub async fn events(pool: &PgPool) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT severity, message FROM events.event_log \
         WHERE message LIKE 'Project widgets:%' ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Issue `number` as GitHub gives it, closed with `reason` when it is set.
pub fn closed(number: u64, reason: &str) -> Value {
    let mut closed = issue(number);
    closed["state"] = json!("closed");
    closed["state_reason"] = json!(reason);
    closed["closed_at"] = json!("2030-05-03T10:00:00Z");
    closed
}

/// A comment as GitHub gives it.
pub fn comment(id: u64, number: u64, text: &str) -> Value {
    json!({
        "id": id, "user": { "login": "anima-app[bot]" }, "body": text,
        "created_at": "2030-05-03T10:00:00Z", "updated_at": "2030-05-03T10:00:00Z",
        "html_url": format!("https://github.com/octo/widgets/issues/{number}#issuecomment-{id}")
    })
}
