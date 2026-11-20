//! The machine's own log of the work that it took (K10 Section 9, SF-012): one entry for each
//! item, also for each item that it refused, written before the state `preparing`. An entry
//! holds no secret: it names the purposes of the secrets, never a value, and no member of the
//! item that this file does not name. Where the log is kept, so that a job cannot write it, is
//! the design of task W2.

use chrono::{DateTime, Utc};
use daemon_core::wire::WorkJob;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

/// What the machine did with the item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The checks passed and the machine starts the item.
    Taken,
    Refused,
}

/// One line of the log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub at: DateTime<Utc>,
    pub id: Uuid,
    pub attempt: u32,
    #[serde(rename = "type")]
    pub work_type: Option<String>,
    pub kind: Option<String>,
    pub project: Option<String>,
    /// `owner/name`.
    pub repository: Option<String>,
    pub commit: Option<String>,
    pub recipe: Option<String>,
    pub runner_version: Option<String>,
    pub runner_sha256: Option<String>,
    /// The purposes of the secrets, never a value.
    pub secrets: Vec<String>,
    pub outcome: Outcome,
    /// The cause of a refusal.
    pub cause: Option<&'static str>,
}

/// The most characters of one value of an entry: an item that the machine refused can hold a
/// value of each length.
const MOST: usize = 128;

fn short(value: Option<&Value>) -> Option<String> {
    Some(value?.as_str()?.chars().take(MOST).collect())
}

/// The entry for the answer `item` of the hub, which the machine may have refused: each value
/// is read from the answer as it came, and cut.
#[must_use]
pub fn entry(
    item: &Value,
    (id, attempt): (Uuid, u32),
    (outcome, cause): (Outcome, Option<&'static str>),
    at: DateTime<Utc>,
) -> Entry {
    let parameters = item.get("parameters");
    let parameter = |name: &str| short(parameters.and_then(|all| all.get(name)));
    let names = |name: &str| short(item.get("repository").and_then(|all| all.get(name)));
    let secrets = item.get("secrets").and_then(Value::as_array);
    Entry {
        at,
        id,
        attempt,
        work_type: short(item.get("type")),
        kind: short(item.get("kind")),
        project: short(item.get("project")),
        repository: names("owner")
            .zip(names("name"))
            .map(|(owner, name)| format!("{owner}/{name}")),
        commit: parameter("commit"),
        recipe: parameter("recipe"),
        runner_version: parameter("runnerVersion"),
        runner_sha256: parameter("runnerSha256"),
        secrets: secrets.map_or_else(Vec::new, |all| {
            all.iter()
                .filter_map(|each| short(Some(each)))
                .take(8)
                .collect()
        }),
        outcome,
        cause,
    }
}

/// The entry of a job that passed each check.
#[must_use]
pub fn taken(job: &WorkJob, at: DateTime<Utc>) -> Entry {
    let item = serde_json::to_value(job).unwrap_or(Value::Null);
    entry(&item, (job.id, job.attempt), (Outcome::Taken, None), at)
}
