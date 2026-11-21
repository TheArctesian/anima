//! The lines that `claude -p --output-format stream-json` writes, read as [`DriveEvent`]s
//! (TH10 Sections 2 and 2b; TH18 Section 2).

use serde_json::Value;

use super::account;
use super::request::{ProgramRequestId, Question, Request, RequestKind};
use super::{DriveEvent, MAX_LINE, ProgramState, TurnOutcome, text_of};
use crate::{MAX_OUTPUT, MAX_TEXT, held};

/// The `system` subtypes that are known and say nothing the view needs.
const QUIET_SYSTEM: [&str; 6] = [
    "status",
    "thinking_tokens",
    "task_started",
    "task_notification",
    "permission_denied",
    "compact_boundary",
];

/// Reads the output of one Claude Code process. It keeps the session ID, so that a change of
/// the ID in the middle of a thread is one [`DriveEvent::Session`] (THR-089).
#[derive(Debug, Default)]
pub struct ClaudeDecoder {
    session: Option<String>,
}

impl ClaudeDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The events of one line. A line that is not JSON, or of a type this decoder does not
    /// know, is [`DriveEvent::Unknown`] that stops the turn (THR-046).
    pub fn read(&mut self, line: &[u8]) -> Vec<DriveEvent> {
        if line.len() > MAX_LINE {
            return vec![unknown("a line over the limit", true)];
        }
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            return vec![unknown("a line that is not JSON", true)];
        };
        let kind = value["type"].as_str().unwrap_or_default();
        match kind {
            "system" => self.system(&value),
            "stream_event" => stream(&value["event"]).into_iter().collect(),
            "assistant" => assistant(&value),
            "user" => user(&value),
            "control_request" => vec![control_request(&value)],
            "control_response" => vec![control_response(&value["response"])],
            "result" => vec![result(&value)],
            // The limits of the account: kept at the machine (THR-051, THR-132).
            "rate_limit_event" => account::claude_limits(&value["rate_limit_info"])
                .map(|facts| DriveEvent::Account {
                    facts: Box::new(facts),
                })
                .into_iter()
                .collect(),
            other => vec![unknown(&format!("type {other}"), true)],
        }
    }

    fn system(&mut self, value: &Value) -> Vec<DriveEvent> {
        let subtype = value["subtype"].as_str().unwrap_or_default();
        match subtype {
            "init" => self.session(value).into_iter().collect(),
            "session_state_changed" => state(value["state"].as_str()).into_iter().collect(),
            quiet if QUIET_SYSTEM.contains(&quiet) => Vec::new(),
            other => vec![unknown(&format!("system {other}"), false)],
        }
    }

    /// A [`DriveEvent::Session`] for the first `init` and for each change of the ID; the `init`
    /// at the start of each later turn of the same session says nothing new.
    fn session(&mut self, value: &Value) -> Option<DriveEvent> {
        let id = value["session_id"].as_str()?.to_owned();
        if self.session.as_deref() == Some(id.as_str()) {
            return None;
        }
        self.session = Some(id.clone());
        Some(DriveEvent::Session {
            id,
            model: value["model"].as_str().map(str::to_owned),
            mode: value["permissionMode"].as_str().map(str::to_owned),
        })
    }
}

fn unknown(what: &str, stops_turn: bool) -> DriveEvent {
    DriveEvent::Unknown {
        what: crate::first_line(what, 200),
        stops_turn,
    }
}

fn state(name: Option<&str>) -> Option<DriveEvent> {
    let state = match name? {
        "running" => ProgramState::Working,
        "requires_action" => ProgramState::NeedsAnswer,
        "idle" => ProgramState::Idle,
        _ => return Some(unknown("a session state", false)),
    };
    Some(DriveEvent::State { state })
}

fn stream(event: &Value) -> Option<DriveEvent> {
    if event["type"].as_str()? != "content_block_delta" {
        return None;
    }
    let delta = &event["delta"];
    match delta["type"].as_str()? {
        "text_delta" => Some(DriveEvent::TextDelta {
            text: text_of(&delta["text"]),
        }),
        "thinking_delta" => Some(DriveEvent::ReasoningDelta {
            text: text_of(&delta["thinking"]),
        }),
        _ => None,
    }
}

fn parent(value: &Value) -> Option<String> {
    value["parent_tool_use_id"].as_str().map(str::to_owned)
}

fn assistant(value: &Value) -> Vec<DriveEvent> {
    let parent = parent(value);
    let blocks = value["message"]["content"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    blocks
        .iter()
        .filter_map(|block| match block["type"].as_str()? {
            "text" => Some(DriveEvent::Message {
                text: held(&text_of(&block["text"]), MAX_TEXT),
                parent: parent.clone(),
            }),
            "tool_use" => Some(DriveEvent::ToolCall {
                id: text_of(&block["id"]),
                name: text_of(&block["name"]),
                input: held(&block["input"].to_string(), MAX_TEXT),
                parent: parent.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// The tool results of a `user` line; a replayed prompt and a plain text are not events.
fn user(value: &Value) -> Vec<DriveEvent> {
    if value["isReplay"].as_bool() == Some(true) {
        return Vec::new();
    }
    let decided_by = value["tool_result_meta"]
        .as_array()
        .and_then(|meta| meta.first())
        .and_then(|meta| meta["permission_decision"]["source"].as_str())
        .map(str::to_owned);
    let blocks = value["message"]["content"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    blocks
        .iter()
        .filter(|block| block["type"] == "tool_result")
        .map(|block| DriveEvent::ToolOutput {
            id: text_of(&block["tool_use_id"]),
            output: held(&result_text(&block["content"]), MAX_OUTPUT),
            failed: block["is_error"].as_bool() == Some(true),
            decided_by: decided_by.clone(),
        })
        .collect()
}

/// The text of a tool result: a string, or the text blocks of a list.
fn result_text(content: &Value) -> String {
    match content.as_array() {
        Some(blocks) => blocks
            .iter()
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        None => text_of(content),
    }
}

fn control_request(value: &Value) -> DriveEvent {
    let request = &value["request"];
    let id = ProgramRequestId::from_json(&value["request_id"]);
    match (request["subtype"].as_str(), id) {
        (Some("can_use_tool"), Some(id)) => DriveEvent::Request(can_use_tool(id, request)),
        _ => unknown("a control request", true),
    }
}

fn can_use_tool(id: ProgramRequestId, request: &Value) -> Request {
    let tool = text_of(&request["tool_name"]);
    let input = request["input"].clone();
    let questions = if tool == "AskUserQuestion" {
        questions(&input["questions"])
    } else {
        Vec::new()
    };
    let kind = match tool.as_str() {
        "AskUserQuestion" => RequestKind::Question,
        "Bash" => RequestKind::Command,
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => RequestKind::FileChange,
        _ => RequestKind::Tool,
    };
    let subject = input["command"]
        .as_str()
        .or_else(|| input["file_path"].as_str())
        .map_or_else(|| input.to_string(), str::to_owned);
    // Only a rule for this session is offered: a rule for a settings file would stay after the
    // thread, and "allow for the session" must not write a file of the program (THR-030).
    let offered: Vec<Value> = request["permission_suggestions"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter(|rule| rule["destination"] == "session")
        .cloned()
        .collect();
    Request {
        id,
        kind,
        tool,
        subject: held(&subject, MAX_TEXT),
        diff: None,
        input,
        session_rules: offered
            .iter()
            .map(|rule| held(&rule.to_string(), 1024))
            .collect(),
        offered,
        questions,
    }
}

fn questions(list: &Value) -> Vec<Question> {
    let list = list.as_array().map_or(&[][..], Vec::as_slice);
    list.iter()
        .map(|question| Question {
            key: text_of(&question["question"]),
            header: question["header"].as_str().map(str::to_owned),
            text: text_of(&question["question"]),
            choices: question["options"]
                .as_array()
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .map(|option| text_of(&option["label"]))
                .collect(),
            many: question["multiSelect"].as_bool() == Some(true),
        })
        .collect()
}

fn control_response(response: &Value) -> DriveEvent {
    match ProgramRequestId::from_json(&response["request_id"]) {
        Some(id) => DriveEvent::Reply {
            id,
            ok: response["subtype"] == "success",
            // The answer to `initialize` names the account: it is dropped (THR-051).
            result: Value::Null,
        },
        None => unknown("a control response with no ID", false),
    }
}

/// The end of a turn: `success` is completed, an abort is an interrupt, each other is a fault.
fn result(value: &Value) -> DriveEvent {
    let reason = value["terminal_reason"].as_str().unwrap_or_default();
    let outcome = match value["subtype"].as_str() {
        Some("success") => TurnOutcome::Completed,
        _ if reason.starts_with("aborted") => TurnOutcome::Interrupted,
        _ => TurnOutcome::Failed,
    };
    let refused = value["permission_denials"].as_array().map_or(0, Vec::len);
    DriveEvent::TurnEnded {
        outcome,
        detail: (!reason.is_empty()).then(|| reason.to_owned()),
        refused: u32::try_from(refused).unwrap_or(u32::MAX),
    }
}
