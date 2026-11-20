//! The lines of a Claude Code session file (`~/.claude/projects/<project>/<id>.jsonl`): a
//! `user` line holds a prompt or the results of tool calls; an `assistant` line holds one block
//! of a reply (text, thinking or a tool call); `custom-title` and `summary` lines name it. A
//! line of a sub-agent (`isSidechain`) and a line the program wrote for itself (`isMeta`) are
//! not a part of the thread.

use serde::Deserialize;
use serde_json::Value;

use super::tool;
use super::{EntryKind, ThreadReader, time};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
    is_sidechain: Option<bool>,
    is_meta: Option<bool>,
    is_compact_summary: Option<bool>,
    custom_title: Option<String>,
    summary: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    model: Option<String>,
    content: Option<Value>,
}

pub(super) fn read(reader: &mut ThreadReader, line: &[u8]) {
    let Ok(line) = serde_json::from_slice::<Line>(line) else {
        return;
    };
    match line.kind.as_deref() {
        Some("custom-title") => {
            if let Some(title) = &line.custom_title {
                reader.name(title);
            }
            return;
        }
        Some("summary") => {
            if let Some(summary) = &line.summary {
                reader.thread.title.get_or_insert_with(|| summary.clone());
            }
            return;
        }
        _ => {}
    }
    if line.is_sidechain == Some(true) || line.is_meta == Some(true) {
        return;
    }
    reader.folder(line.cwd.as_deref(), line.git_branch.as_deref());
    let at = time(line.timestamp.as_deref());
    let Some(message) = line.message else {
        return;
    };
    match line.kind.as_deref() {
        Some("user") if line.is_compact_summary == Some(true) => {
            reader.push(at, EntryKind::Compacted);
        }
        Some("user") => user(reader, at, message.content.as_ref()),
        Some("assistant") => {
            reader.model(message.model.as_deref());
            assistant(reader, at, message.content.as_ref());
        }
        _ => {}
    }
}

/// A prompt the person typed, or the results of tool calls.
fn user(reader: &mut ThreadReader, at: Option<super::Time>, content: Option<&Value>) {
    match content {
        Some(Value::String(text)) if typed(text) => reader.prompt(at, text, 0),
        Some(Value::Array(blocks)) => {
            let mut text = Vec::new();
            let mut images = 0;
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => text.extend(block.get("text").and_then(Value::as_str)),
                    Some("image") => images += 1,
                    Some("tool_result") => result(reader, block),
                    _ => {}
                }
            }
            let text = text.join("\n");
            if typed(&text) || (text.is_empty() && images > 0) {
                reader.prompt(at, &text, images);
            }
        }
        _ => {}
    }
}

/// Text the person typed: not text the program wrote in the person's turn, which starts with a
/// tag of its own, except a slash command or a shell line the person typed.
fn typed(text: &str) -> bool {
    const TYPED_TAGS: [&str; 2] = ["<command-name>", "<bash-input>"];
    let text = text.trim_start();
    !text.is_empty()
        && (!text.starts_with('<') || TYPED_TAGS.iter().any(|tag| text.starts_with(tag)))
}

fn result(reader: &mut ThreadReader, block: &Value) {
    let Some(call) = block.get("tool_use_id").and_then(Value::as_str) else {
        return;
    };
    let failed = block.get("is_error").and_then(Value::as_bool) == Some(true);
    let output = match block.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    reader.result(call, &output, failed);
}

fn assistant(reader: &mut ThreadReader, at: Option<super::Time>, content: Option<&Value>) {
    let Some(Value::Array(blocks)) = content else {
        if let Some(Value::String(text)) = content {
            reader.text(at, false, text);
        }
        return;
    };
    for block in blocks {
        let field = |name: &str| block.get(name).and_then(Value::as_str).unwrap_or_default();
        match field("type") {
            "text" => reader.text(at, false, field("text")),
            "thinking" => reader.text(at, true, field("thinking")),
            "tool_use" | "server_tool_use" => {
                let name = field("name");
                let input = block.get("input").cloned().unwrap_or(Value::Null);
                let raw = serde_json::to_string_pretty(&input).unwrap_or_default();
                reader.call(at, tool::call(field("id"), name, &input, &raw));
            }
            _ => {}
        }
    }
}
