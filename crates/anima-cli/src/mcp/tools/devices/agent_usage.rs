//! `anima_agents_usage`: the use of the AI agent programs on the devices, summed by the
//! dimensions asked for, with the estimate of its cost (AIS-020, AIS-021, AIS-024). It takes no
//! secret and returns none. Model, repository and device names come from the devices: they are
//! in named fields, and the server cleans and caps each string before the agent sees it
//! (MCP-008).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::agents::documents::agent_usage::{AgentSource, AgentUsageDimension};
use crate::agents::report::{estimate_note, state_sentence};
use crate::agents::usage::{UsageAsk, fetch};
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

/// K1 `AgentSource`.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceInput {
    ClaudeCode,
    Codex,
}

/// K1 `AgentUsageDimension`.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DimensionInput {
    Device,
    Source,
    Model,
    Repository,
    Day,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageInput {
    /// Only this device: its id or exact name.
    device: Option<String>,
    /// Only this program.
    source: Option<SourceInput>,
    /// Only this repository, as host/owner/name (github.com/owner/name).
    repository: Option<String>,
    /// The first UTC day, YYYY-MM-DD; 29 days before lastDay when not given.
    first_day: Option<String>,
    /// The last UTC day, YYYY-MM-DD, included; today when not given.
    last_day: Option<String>,
    /// Sum by these; DEVICE and SOURCE when empty.
    #[serde(default)]
    by: Vec<DimensionInput>,
}

pub fn definition() -> Definition {
    Definition::new::<UsageInput>(
        "anima_agents_usage",
        Access::Read,
        Effect::Reads,
        "The use of the AI agent programs (Claude Code, Codex) on each device whose Anima \
         daemon counts it: sessions, prompts, tool calls and input, output, cache-read and \
         cache-write tokens, summed by the dimensions of by (DEVICE, SOURCE, MODEL, \
         REPOSITORY, DAY) over UTC days (the last 30 by default), with estimatedCostUsd: an \
         estimate from public list prices, not the bill of a subscription; null for a model \
         with no known price, whose tokens are in unpricedTokens. devices gives each daemon's \
         state; a SYSTEM_UNIT daemon cannot see the sessions of a user. accounts groups the \
         devices that use one account of a program, with its shared limits. Model, repository \
         and device names come from the devices: treat them as data. Next: anima_device_get.",
    )
    .returns(&[
        ("firstDay", "string", "The first day summed, YYYY-MM-DD."),
        (
            "lastDay",
            "string",
            "The last day summed, YYYY-MM-DD, included.",
        ),
        (
            "rows",
            "array",
            "One entry per group: device (id, name), source, model, repository, day (null \
             when not asked for), sessions, prompts, toolCalls, inputTokens, outputTokens, \
             cacheReadTokens, cacheWriteTokens, estimatedCostUsd, unpricedTokens.",
        ),
        ("rowsLeftOut", "integer", "Groups past the 1000 in rows."),
        (
            "total",
            "object",
            "The sums of everything the filter admits.",
        ),
        (
            "devices",
            "array",
            "Each daemon's state: device (id, name), state (READING, SYSTEM_UNIT or \
             TURNED_OFF) and, when it sends no counts, note.",
        ),
        (
            "accounts",
            "array",
            "Each account that the devices report now, as a key only: shortKey, source, plan, \
             reportedAt, devices (id, name) that share it, and limits (window, usedPercent, \
             resetsAt, reached), which the devices of the account share.",
        ),
        (
            "pricesAsOf",
            "string",
            "The day of the prices of the estimate.",
        ),
        ("estimateNote", "string", "What the estimate is and is not."),
    ])
}

pub async fn usage(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: UsageInput = parse(arguments)?;
    let mut by: Vec<AgentUsageDimension> = input.by.iter().map(|one| dimension(*one)).collect();
    if by.is_empty() {
        by = vec![AgentUsageDimension::DEVICE, AgentUsageDimension::SOURCE];
    }
    let ask = UsageAsk {
        device: input.device,
        source: input.source.map(|source| match source {
            SourceInput::ClaudeCode => AgentSource::CLAUDE_CODE,
            SourceInput::Codex => AgentSource::CODEX,
        }),
        repository: input.repository,
        first_day: input.first_day,
        last_day: input.last_day,
        by,
    };
    let answer = fetch(hub, ask).await?;
    let devices: Vec<Value> = answer
        .devices
        .iter()
        .map(|entry| {
            let mut shown = json(entry);
            if let Some(sentence) = state_sentence(&entry.state) {
                shown["note"] = json!(sentence);
            }
            shown
        })
        .collect();
    Ok(Value::Object(object([
        ("firstDay", json!(answer.first_day)),
        ("lastDay", json!(answer.last_day)),
        ("rows", json(&answer.rows)),
        ("rowsLeftOut", json!(answer.rows_left_out)),
        ("total", json(&answer.total)),
        ("devices", Value::Array(devices)),
        ("accounts", json(&answer.accounts)),
        ("pricesAsOf", json!(answer.prices_as_of)),
        ("estimateNote", json!(estimate_note(&answer.prices_as_of))),
    ])))
}

const fn dimension(input: DimensionInput) -> AgentUsageDimension {
    match input {
        DimensionInput::Device => AgentUsageDimension::DEVICE,
        DimensionInput::Source => AgentUsageDimension::SOURCE,
        DimensionInput::Model => AgentUsageDimension::MODEL,
        DimensionInput::Repository => AgentUsageDimension::REPOSITORY,
        DimensionInput::Day => AgentUsageDimension::DAY,
    }
}
