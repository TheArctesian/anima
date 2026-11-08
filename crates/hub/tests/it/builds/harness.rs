//! A signed-in operator, enrolled builders and releases, through the hub's `/graphql` edge.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start};

pub const LINUX: &str = "{ os: LINUX, arch: X86_64 }";
pub const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// The data of a response that must have no errors.
pub fn data(response: Value) -> Value {
    assert!(response["errors"].is_null(), "{response}");
    response["data"].clone()
}

/// The operator's session token, from a hub at `start()`.
pub async fn operator(pool: &PgPool) -> String {
    set_operator(pool).await;
    Hub::new(pool, start()).token().await
}

/// An enrolled builder: its id and its token.
pub struct Enrolled {
    pub id: String,
    pub token: String,
}

/// Enrols a builder for linux/x86_64 from the hub's client address.
pub async fn enrol(hub: &Hub, operator: &str, name: &str) -> Enrolled {
    let query = format!(
        "mutation($name: String!) {{ enrolBuilder(name: $name, platform: {LINUX}) \
         {{ token builder {{ id }} }} }}"
    );
    let enrolment = data(hub.post(Some(operator), &query, json!({ "name": name })).await);
    let enrolment = &enrolment["enrolBuilder"];
    Enrolled {
        id: enrolment["builder"]["id"].as_str().unwrap().to_owned(),
        token: enrolment["token"].as_str().unwrap().to_owned(),
    }
}

pub const CREATE: &str = "mutation($version: String!, $commit: String!) { \
    createRelease(version: $version, commit: $commit, targets: [LINUX_X86_64]) \
    { id version jobs { id state } } }";

/// Creates a linux release of `version`; returns the whole response.
pub async fn create(hub: &Hub, operator: &str, version: &str) -> Value {
    let variables = json!({ "version": version, "commit": COMMIT });
    hub.post(Some(operator), CREATE, variables).await
}

/// Creates a linux release of `version`; returns its one job's id.
pub async fn create_job(hub: &Hub, operator: &str, version: &str) -> String {
    let release = data(create(hub, operator, version).await);
    release["createRelease"]["jobs"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Claims a linux job with `token`; returns the whole response.
pub async fn claim(hub: &Hub, token: &str) -> Value {
    let query = format!(
        "mutation {{ claimBuildJob(platform: {LINUX}) {{ id state attempts builderName }} }}"
    );
    hub.post(Some(token), &query, json!({})).await
}

pub const REPORT: &str = "mutation($id: ID!, $state: BuildJobState, $lines: [String!]!) { \
    reportBuildJob(id: $id, state: $state, logLines: $lines) { id state leaseEndsAt } }";

/// Reports on job `id` with `token`; returns the whole response.
pub async fn report(hub: &Hub, token: &str, id: &str, lines: &[&str]) -> Value {
    let variables = json!({ "id": id, "state": "BUILDING", "lines": lines });
    hub.post(Some(token), REPORT, variables).await
}

/// A hub whose clock reads `start()` plus `seconds`.
pub fn hub_at(pool: &PgPool, seconds: i64) -> Hub {
    Hub::new(pool, at(seconds))
}

pub fn at(seconds: i64) -> DateTime<Utc> {
    start() + chrono::TimeDelta::seconds(seconds)
}
