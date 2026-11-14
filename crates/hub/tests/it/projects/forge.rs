//! Live reads of a project repository through GraphQL against the loopback GitHub (K-E1
//! Section 13; PRJ-027 to PRJ-035): pull requests, issues and workflow runs in K1's shapes,
//! the repository's access, the cache, GitHub's failures as fixed words, the token kinds, and
//! GitHub's text passed through unchanged and never logged.

mod access;
mod cache;
mod changes;
mod failures;
mod issues;
mod leaks;
mod pull_requests;
mod tokens;
mod workflows;

use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use anima_hub::clock::Clock;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::github::{Repo, files, list};
use super::{Kit, kit_at};
use crate::{hub::start, sources::stub::Reply};

pub const NAME: &str = "octo/widgets";
/// Every live read of the repository is below this path.
pub const BASE: &str = "/repos/octo/widgets";

/// A clock the test moves on; it starts at `start()`.
#[derive(Clone, Default)]
pub struct Moving(Arc<AtomicI64>);

impl Clock for Moving {
    fn now(&self) -> DateTime<Utc> {
        start() + TimeDelta::seconds(self.0.load(Ordering::SeqCst))
    }
}

impl Moving {
    pub fn advance(&self, seconds: i64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}

/// A signed-in hub with Projects on and one READY repository, `octo/widgets`.
pub struct Forge {
    pub kit: Kit,
    pub clock: Moving,
    /// `ProjectRepository.id` of `octo/widgets`.
    pub repository: String,
}

impl Forge {
    /// Posts `query` as the signed-in operator, with `$repository` set.
    pub async fn ask(&self, query: &str, mut variables: Value) -> Value {
        variables["repository"] = json!(self.repository);
        self.kit.post(query, variables).await
    }

    /// GitHub answers `BASE` + `tail` with `reply`.
    pub fn reply(&self, tail: &str, reply: Reply) {
        self.kit.stub.reply(&format!("{BASE}{tail}"), reply);
    }

    /// GitHub answers `BASE` + `tail` with 200 and `body`.
    pub fn ok(&self, tail: &str, body: &Value) {
        self.reply(tail, ok(&body.to_string()));
    }

    /// The GET requests for `BASE` + `tail` (any query) so far, with their queries.
    pub fn heard(&self, tail: &str) -> Vec<String> {
        let path = format!("{BASE}{tail}");
        self.kit
            .stub
            .gets()
            .into_iter()
            .filter(|heard| heard.split('?').next() == Some(path.as_str()))
            .collect()
    }

    /// Every GET request below `BASE` so far but the sync's tree and deployments.
    pub fn live_reads(&self) -> usize {
        self.kit
            .stub
            .gets()
            .iter()
            .filter(|heard| heard.starts_with(&format!("{BASE}/")))
            .filter(|heard| !heard.contains("/git/trees/") && !heard.contains("/deployments"))
            .count()
    }
}

/// The repository synced once from the stub, at `start()`.
pub async fn forge(pool: &PgPool) -> Forge {
    let clock = Moving::default();
    let kit = kit_at(pool, Arc::new(clock.clone())).await;
    let account = kit.connect().await;
    files(&kit.stub, NAME, &[], &[]);
    list(&kit.stub, &[Repo::new(1, NAME)]);
    kit.run(account).await;
    let project = kit.project("widgets").await;
    let repository = project["repositories"][0]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("no repository: {project}"))
        .to_owned();
    Forge {
        kit,
        clock,
        repository,
    }
}

/// 200 with `body`.
pub fn ok(body: &str) -> Reply {
    reply(200, &[], body)
}

pub fn reply(status: u16, headers: &[(&'static str, &str)], body: &str) -> Reply {
    Reply {
        status,
        headers: headers.iter().map(|(n, v)| (*n, (*v).to_owned())).collect(),
        body: body.to_owned(),
    }
}

/// A sha of 40 hex digits made from `n`.
pub fn sha(n: u64) -> String {
    format!("{n:040x}")
}

/// A pull request as GitHub lists it, from this repository, open unless `state` says not.
pub fn pull(number: u64, state: &str) -> Value {
    let side = |branch: &str| json!({ "ref": branch, "sha": sha(number), "repo": { "id": 1, "full_name": NAME } });
    json!({
        "number": number, "title": format!("Pull {number}"), "state": state, "draft": false,
        "user": { "login": "octo" }, "head": side("feature"), "base": side("main"),
        "labels": [], "created_at": "2030-05-01T10:00:00Z", "updated_at": "2030-05-02T10:00:00Z",
        "merged_at": null, "html_url": format!("https://github.com/{NAME}/pull/{number}"),
        "body": null
    })
}

/// An issue as GitHub lists it.
pub fn issue(number: u64) -> Value {
    json!({
        "number": number, "title": format!("Issue {number}"), "state": "open", "state_reason": null,
        "user": { "login": "octo" }, "labels": [], "assignees": [], "comments": 0,
        "created_at": "2030-05-01T10:00:00Z", "updated_at": "2030-05-02T10:00:00Z",
        "closed_at": null, "html_url": format!("https://github.com/{NAME}/issues/{number}"),
        "body": null
    })
}

/// A pull request as GitHub's issue list gives it: an issue with the `pull_request` key.
pub fn pull_as_issue(number: u64) -> Value {
    let mut entry = issue(number);
    entry["pull_request"] = json!({ "url": "https://api.github.com/x" });
    entry
}

/// A completed, successful run of this repository.
pub fn run(id: u64) -> Value {
    json!({
        "id": id, "workflow_id": 7, "name": "CI", "run_number": id, "run_attempt": 1,
        "event": "push", "status": "completed", "conclusion": "success", "head_branch": "main",
        "head_sha": sha(id), "head_commit": { "message": "Fix\n\nmore" },
        "actor": { "login": "octo" }, "repository": { "id": 1 }, "head_repository": { "id": 1 },
        "created_at": "2030-05-01T10:00:00Z", "updated_at": "2030-05-01T10:05:00Z",
        "run_started_at": "2030-05-01T10:00:10Z",
        "html_url": format!("https://github.com/{NAME}/actions/runs/{id}")
    })
}

/// The nodes of the connection at `response.data[root]`.
pub fn nodes(response: &Value, root: &str) -> Vec<Value> {
    response["data"][root]["edges"]
        .as_array()
        .unwrap_or_else(|| panic!("no {root}: {response}"))
        .iter()
        .map(|edge| edge["node"].clone())
        .collect()
}

/// The `number` of each of `nodes`.
pub fn numbers(nodes: &[Value]) -> Vec<u64> {
    nodes
        .iter()
        .map(|node| node["number"].as_u64().unwrap())
        .collect()
}
