//! The use of the AI agent programs through the hub's routes (R-135; AIS-018 to AIS-024; K8
//! Section 4): a daemon pushes `agentUsage` in its batch, and `agentUsage` sums it.

mod accounts;
mod guard;
mod query;
mod sessions;
mod storage;

use axum::http::StatusCode;
use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use serde_json::{Value, json};

use crate::daemon::{batches::cpu_batch, harness::Daemons};

pub const USAGE: &str = "query($filter: AgentUsageFilter, $by: [AgentUsageDimension!]!) {
    agentUsage(filter: $filter, by: $by) {
        firstDay lastDay rowsLeftOut pricesAsOf
        rows { ...sums device { name } source model repository day }
        total { ...sums device { name } source model repository day }
        devices { device { name } state } } }
    fragment sums on AgentUsageRow {
        sessions prompts toolCalls inputTokens outputTokens cacheReadTokens cacheWriteTokens
        estimatedCostUsd unpricedTokens }";

const CREATE_KEY: &str =
    "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";

/// One model's tokens of a session day: input, output, cache read, cache write.
pub fn tokens(model: &str, [input, output, cache_read, cache_write]: [u64; 4]) -> Value {
    json!({ "model": model, "inputTokens": input, "outputTokens": output,
            "cacheReadTokens": cache_read, "cacheWriteTokens": cache_write })
}

/// One session day of the session `(source, id)` on `day` with its prompts and tool calls, the
/// session from 08:00 to 09:00 of that day.
pub fn entry(
    (source, id): (&str, &str),
    day: NaiveDate,
    counts: (u64, u64),
    models: Value,
) -> Value {
    let at = |hour: u32| day.and_hms_opt(hour, 0, 0).unwrap().and_utc();
    let mut entry = json!({ "source": source, "id": id, "day": day.to_string(),
            "startedAt": at(8).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "endedAt": at(9).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "prompts": counts.0, "toolCalls": counts.1 });
    entry["models"] = models;
    entry
}

/// `entry` with its repository.
pub fn in_repository(mut entry: Value, repository: &str) -> Value {
    entry["repository"] = json!(repository);
    entry
}

/// The daemon of `device` and its token, with the hub's clock.
pub struct Pusher<'a> {
    pub daemons: &'a Daemons,
    pub device: &'a str,
    pub token: String,
    pub now: DateTime<Utc>,
    sent: std::cell::Cell<i64>,
}

impl<'a> Pusher<'a> {
    pub async fn new(daemons: &'a Daemons, device: &'a str, now: DateTime<Utc>) -> Self {
        let token = daemons.token_for(device).await;
        Self {
            daemons,
            device,
            token,
            now,
            sent: std::cell::Cell::new(0),
        }
    }

    /// Pushes a batch with one CPU point and `usage`; the status. Each batch has a later point.
    pub async fn push(&self, state: &str, session_days: Value) -> StatusCode {
        let step = self.sent.get();
        self.sent.set(step + 1);
        let at = self.now - TimeDelta::seconds(600 - 10 * step);
        let mut batch = cpu_batch(self.device, at, 5.0);
        batch["agentUsage"] = json!({ "state": state, "sessionDays": session_days });
        self.daemons.push(&self.token, &batch).await.0
    }

    /// As `push`, and asserts that the hub took the batch.
    pub async fn reading(&self, session_days: Value) {
        assert_eq!(
            self.push("reading", session_days).await,
            StatusCode::NO_CONTENT
        );
    }
}

/// The `agentUsage` answer to the signed-in operator, or the whole response when it failed.
pub async fn usage(daemons: &Daemons, filter: Value, by: &[&str]) -> Value {
    let answer = daemons
        .operate(USAGE, json!({ "filter": filter, "by": by }))
        .await;
    if answer["errors"].is_array() {
        return answer;
    }
    answer["data"]["agentUsage"].clone()
}

/// A read key's secret, made by the signed-in operator.
pub async fn read_key(daemons: &Daemons, now: DateTime<Utc>) -> String {
    let input = json!({ "name": "usage reader", "scope": "READ", "madeFor": "AGENT",
                        "expiresAt": now + TimeDelta::days(30) });
    let made = daemons.operate(CREATE_KEY, json!({ "input": input })).await;
    made["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned()
}

/// The counts of a row, without its keys and estimate.
pub fn counts(row: &Value) -> [f64; 7] {
    [
        "prompts",
        "toolCalls",
        "inputTokens",
        "outputTokens",
        "cacheReadTokens",
        "cacheWriteTokens",
        "sessions",
    ]
    .map(|name| {
        row[name]
            .as_f64()
            .unwrap_or_else(|| panic!("{name}: {row}"))
    })
}
