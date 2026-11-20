//! The lines of a Codex rollout (`~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`):
//! `session_meta` names the thread, its folder and its branch; `turn_context` the model; the
//! `event_msg` lines `user_message` and `agent_message` hold the prompts and the replies as the
//! person saw them; the `response_item` lines hold the reasoning and the tool calls with their
//! outputs; `task_complete` and `turn_aborted` end a turn. The `response_item` messages repeat
//! the prompts and replies with the text the program adds, so they are not read.

use serde::Deserialize;
use serde_json::Value;

use super::tool;
use super::{EntryKind, ThreadReader, Time, time};

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    payload: Option<Value>,
}

pub(super) fn read(reader: &mut ThreadReader, line: &[u8]) {
    let Ok(Line {
        kind,
        timestamp,
        payload: Some(payload),
    }) = serde_json::from_slice::<Line>(line)
    else {
        return;
    };
    let at = time(timestamp.as_deref());
    let field = |name: &str| payload.get(name).and_then(Value::as_str);
    match kind.as_deref() {
        Some("session_meta") => {
            if let Some(id) = field("id") {
                reader.thread.id = Some(id.to_owned());
            }
            let branch = payload.pointer("/git/branch").and_then(Value::as_str);
            reader.folder(field("cwd"), branch);
            reader.at(at);
        }
        Some("turn_context") => {
            reader.model(field("model"));
            reader.folder(field("cwd"), None);
        }
        Some("event_msg") => event(reader, at, &payload),
        Some("response_item") => response(reader, at, &payload),
        Some("compacted") => {
            reader.push(at, EntryKind::Compacted);
        }
        _ => {}
    }
}

fn event(reader: &mut ThreadReader, at: Option<Time>, payload: &Value) {
    let field = |name: &str| {
        payload
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    match field("type") {
        "user_message" => {
            let images = ["images", "local_images"]
                .iter()
                .filter_map(|name| payload.get(name)?.as_array().map(Vec::len))
                .sum::<usize>();
            reader.prompt(
                at,
                field("message"),
                u32::try_from(images).unwrap_or(u32::MAX),
            );
        }
        "agent_message" => reader.text(at, false, field("message")),
        "task_complete" => {
            reader.push(at, EntryKind::TurnEnd { stopped: false });
        }
        "turn_aborted" => {
            reader.push(at, EntryKind::TurnEnd { stopped: true });
        }
        "thread_name_updated" => reader.name(field("thread_name")),
        _ => {}
    }
}

fn response(reader: &mut ThreadReader, at: Option<Time>, payload: &Value) {
    let field = |name: &str| {
        payload
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    match field("type") {
        "reasoning" => {
            let summary = payload
                .get("summary")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n\n");
            reader.text(at, true, &summary);
        }
        "function_call" | "custom_tool_call" => {
            let raw = payload
                .get("arguments")
                .or_else(|| payload.get("input"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let input = serde_json::from_str(raw).unwrap_or(Value::Null);
            reader.call(at, tool::call(field("call_id"), field("name"), &input, raw));
        }
        "local_shell_call" => {
            let input = payload.get("action").cloned().unwrap_or(Value::Null);
            let raw = serde_json::to_string_pretty(&input).unwrap_or_default();
            reader.call(
                at,
                tool::call(field("call_id"), "local_shell", &input, &raw),
            );
        }
        "web_search_call" => {
            let input = payload.get("action").cloned().unwrap_or(Value::Null);
            let raw = serde_json::to_string_pretty(&input).unwrap_or_default();
            reader.call(at, tool::call(field("id"), "web_search", &input, &raw));
        }
        "function_call_output" | "custom_tool_call_output" => output(reader, payload),
        _ => {}
    }
}

/// The output of a call: a text, a list of parts with text, or an object with `content`; a
/// call failed when its output says so (`success: false`, or an exit code that is not 0).
fn output(reader: &mut ThreadReader, payload: &Value) {
    let Some(call) = payload.get("call_id").and_then(Value::as_str) else {
        return;
    };
    let value = payload.get("output").unwrap_or(&Value::Null);
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(_) => value
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    };
    let parsed: Option<Value> = serde_json::from_str(&text).ok();
    let inner = parsed.as_ref().unwrap_or(value);
    let failed = inner.get("success").and_then(Value::as_bool) == Some(false)
        || inner
            .pointer("/metadata/exit_code")
            .and_then(Value::as_i64)
            .is_some_and(|code| code != 0);
    let shown = parsed
        .as_ref()
        .and_then(|parsed| parsed.get("output").and_then(Value::as_str))
        .map_or(text.clone(), str::to_owned);
    reader.result(call, &shown, failed);
}
