//! The JSON-RPC lines that `codex app-server` writes, read as [`DriveEvent`]s (TH10 Section
//! 3.2, TH18 Section 3).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::account;
use super::request::{ProgramRequestId, Question, Request, RequestKind};
use super::{DriveEvent, MAX_LINE, ProgramState, TurnOutcome, text_of};
use crate::{MAX_OUTPUT, MAX_TEXT, held};

/// Notifications that are known and say nothing the view needs.
const QUIET: [&str; 8] = [
    "thread/tokenUsage/updated",
    "mcpServer/startupStatus/updated",
    "remoteControl/status/changed",
    "turn/diff/updated",
    "thread/goal/cleared",
    "item/reasoning/summaryPartAdded",
    "item/userMessage/delta",
    "thread/name/updated",
];

/// Reads the output of one `codex app-server`. It keeps the open tool items, so that a turn that
/// ends closes them (TH10: an interrupt sends no `item/completed`), and the changes of each file
/// item, so that an approval shows its diff.
#[derive(Debug, Default)]
pub struct CodexDecoder {
    open: BTreeSet<String>,
    changes: BTreeMap<String, String>,
}

impl CodexDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The events of one line. A server request of a kind this decoder does not know, or a line
    /// that is not JSON-RPC, is [`DriveEvent::Unknown`] that stops the turn (THR-046).
    pub fn read(&mut self, line: &[u8]) -> Vec<DriveEvent> {
        if line.len() > MAX_LINE {
            return vec![unknown("a line over the limit", true)];
        }
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            return vec![unknown("a line that is not JSON", true)];
        };
        let id = value.get("id").and_then(ProgramRequestId::from_json);
        if value.get("id").is_some_and(|given| !given.is_null()) && id.is_none() {
            return vec![unknown(
                "an ID that is not a string or a whole number",
                true,
            )];
        }
        match (value["method"].as_str(), id) {
            (Some(method), Some(id)) => vec![self.server_request(method, id, &value["params"])],
            (Some(method), None) => self.notification(method, &value["params"]),
            (None, Some(id)) => replied(id, &value),
            (None, None) => vec![unknown("a line with no method and no ID", true)],
        }
    }

    fn notification(&mut self, method: &str, params: &Value) -> Vec<DriveEvent> {
        match method {
            "thread/started" => vec![session(&params["thread"])],
            "turn/started" => vec![
                DriveEvent::TurnStarted {
                    turn: params["turn"]["id"].as_str().map(str::to_owned),
                },
                DriveEvent::State {
                    state: ProgramState::Working,
                },
            ],
            "thread/status/changed" => vec![status(&params["status"])],
            "item/agentMessage/delta" => vec![DriveEvent::TextDelta {
                text: text_of(&params["delta"]),
            }],
            "item/reasoning/summaryTextDelta" | "item/reasoning/textDelta" => {
                vec![DriveEvent::ReasoningDelta {
                    text: text_of(&params["delta"]),
                }]
            }
            "item/commandExecution/outputDelta" => vec![DriveEvent::ToolOutputDelta {
                id: text_of(&params["itemId"]),
                text: held(&text_of(&params["delta"]), MAX_OUTPUT),
            }],
            "item/started" => self.item_started(&params["item"]).into_iter().collect(),
            "item/completed" => self.item_completed(&params["item"]).into_iter().collect(),
            "serverRequest/resolved" => ProgramRequestId::from_json(&params["requestId"])
                .map(|id| DriveEvent::RequestClosed { id })
                .into_iter()
                .collect(),
            "turn/completed" => self.turn_completed(&params["turn"]),
            "configWarning" => vec![DriveEvent::Notice {
                text: held(&text_of(&params["summary"]), MAX_TEXT),
            }],
            // The account, the plan and the limits: kept at the machine (THR-051, THR-132).
            "account/updated" => facts(account::codex_account_updated(params)),
            "account/rateLimits/updated" => facts(account::codex_limits(&params["rateLimits"])),
            quiet if QUIET.contains(&quiet) => Vec::new(),
            other => vec![unknown(&format!("notification {other}"), false)],
        }
    }

    fn item_started(&mut self, item: &Value) -> Option<DriveEvent> {
        let id = text_of(&item["id"]);
        let (name, input) = match item["type"].as_str()? {
            "commandExecution" => ("command", text_of(&item["command"])),
            "fileChange" => {
                let diff = changes(&item["changes"]);
                self.changes.insert(id.clone(), diff.clone());
                ("patch", diff)
            }
            "mcpToolCall" => (
                "mcp",
                format!("{} {}", text_of(&item["server"]), text_of(&item["tool"])),
            ),
            _ => return None,
        };
        self.open.insert(id.clone());
        Some(DriveEvent::ToolCall {
            id,
            name: name.to_owned(),
            input: held(&input, MAX_TEXT),
            parent: None,
        })
    }

    fn item_completed(&mut self, item: &Value) -> Option<DriveEvent> {
        let id = text_of(&item["id"]);
        match item["type"].as_str()? {
            "agentMessage" => Some(DriveEvent::Message {
                text: held(&text_of(&item["text"]), MAX_TEXT),
                parent: None,
            }),
            "commandExecution" | "fileChange" | "mcpToolCall" => {
                self.open.remove(&id);
                self.changes.remove(&id);
                let status = item["status"].as_str().unwrap_or_default();
                Some(DriveEvent::ToolOutput {
                    id,
                    output: held(&text_of(&item["aggregatedOutput"]), MAX_OUTPUT),
                    failed: status != "completed"
                        || item["exitCode"].as_i64().is_some_and(|code| code != 0),
                    decided_by: (status == "declined").then(|| "user_reject".to_owned()),
                })
            }
            _ => None,
        }
    }

    fn turn_completed(&mut self, turn: &Value) -> Vec<DriveEvent> {
        let mut events: Vec<DriveEvent> = std::mem::take(&mut self.open)
            .into_iter()
            .map(|id| DriveEvent::ToolOutput {
                id,
                output: "The turn ended before this call ended.".to_owned(),
                failed: true,
                decided_by: None,
            })
            .collect();
        self.changes.clear();
        let outcome = match turn["status"].as_str() {
            Some("completed") => TurnOutcome::Completed,
            Some("interrupted") => TurnOutcome::Interrupted,
            _ => TurnOutcome::Failed,
        };
        let detail = turn["error"]["message"]
            .as_str()
            .map(|text| held(text, MAX_TEXT));
        events.push(DriveEvent::TurnEnded {
            outcome,
            detail,
            refused: 0,
        });
        events
    }

    fn server_request(&self, method: &str, id: ProgramRequestId, params: &Value) -> DriveEvent {
        let item = text_of(&params["itemId"]);
        let (kind, subject, diff) = match method {
            "item/commandExecution/requestApproval" => {
                (RequestKind::Command, text_of(&params["command"]), None)
            }
            "item/fileChange/requestApproval" => {
                let diff = self.changes.get(&item).cloned();
                (RequestKind::FileChange, text_of(&params["reason"]), diff)
            }
            "item/tool/requestUserInput" => (RequestKind::Question, String::new(), None),
            other => return unknown(&format!("server request {other}"), true),
        };
        let offered: Vec<Value> = params["availableDecisions"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            // Only the decision for this session: an amendment of the exec policy is a rule that
            // stays after the thread (THR-030).
            .filter(|decision| decision.as_str() == Some("acceptForSession"))
            .cloned()
            .collect();
        DriveEvent::Request(Request {
            id,
            kind,
            tool: item,
            subject: held(&subject, MAX_TEXT),
            diff,
            input: params.clone(),
            session_rules: offered
                .iter()
                .map(|rule| held(&rule.to_string(), 1024))
                .collect(),
            offered,
            questions: questions(&params["questions"]),
        })
    }
}

fn unknown(what: &str, stops_turn: bool) -> DriveEvent {
    DriveEvent::Unknown {
        what: crate::first_line(what, 200),
        stops_turn,
    }
}

fn session(thread: &Value) -> DriveEvent {
    DriveEvent::Session {
        id: text_of(&thread["id"]),
        model: None,
        mode: None,
    }
}

fn status(status: &Value) -> DriveEvent {
    let waiting = status["activeFlags"].as_array().is_some_and(|flags| {
        flags
            .iter()
            .any(|flag| flag.as_str().is_some_and(|f| f.starts_with("waitingOn")))
    });
    let state = match status["type"].as_str() {
        Some("idle") => ProgramState::Idle,
        Some("active") if waiting => ProgramState::NeedsAnswer,
        Some("active") => ProgramState::Working,
        _ => return unknown("a thread status", false),
    };
    DriveEvent::State { state }
}

/// The paths and diffs of a file change item, as one text.
fn changes(list: &Value) -> String {
    let list = list.as_array().map_or(&[][..], Vec::as_slice);
    list.iter()
        .map(|change| format!("{}\n{}", text_of(&change["path"]), text_of(&change["diff"])))
        .collect::<Vec<_>>()
        .join("\n")
}

fn questions(list: &Value) -> Vec<Question> {
    let list = list.as_array().map_or(&[][..], Vec::as_slice);
    list.iter()
        .map(|question| Question {
            key: text_of(&question["id"]),
            header: question["header"].as_str().map(str::to_owned),
            text: text_of(&question["question"]),
            choices: question["options"]
                .as_array()
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .map(|option| text_of(&option["label"]))
                .collect(),
            many: false,
        })
        .collect()
}

/// The account facts of one report as an event, when it gave any.
fn facts(found: Option<account::AccountFacts>) -> Vec<DriveEvent> {
    found
        .map(|facts| DriveEvent::Account {
            facts: Box::new(facts),
        })
        .into_iter()
        .collect()
}

/// The reply, and the account facts of the answer to `account/read` (it has
/// `requiresOpenaiAuth`) or to `account/rateLimits/read`. The reply itself keeps only IDs.
fn replied(id: ProgramRequestId, value: &Value) -> Vec<DriveEvent> {
    let result = &value["result"];
    let mut events = if result.get("requiresOpenaiAuth").is_some() {
        facts(account::codex_account(result))
    } else if result.get("rateLimits").is_some() {
        facts(account::codex_limits(&result["rateLimits"]))
    } else {
        Vec::new()
    };
    events.insert(0, reply(id, value));
    events
}

/// The answer to a call of the runner. Only the IDs of the thread and of the turn are kept, and
/// of an error its code and message: the answer to `initialize` names the home folder, and the
/// account facts of `account/read` go only into [`DriveEvent::Account`] (THR-051).
fn reply(id: ProgramRequestId, value: &Value) -> DriveEvent {
    let ok = value.get("error").is_none();
    let result = if ok {
        let mut kept = serde_json::Map::new();
        for key in ["thread", "turn"] {
            if let Some(given) = value["result"][key]["id"].as_str() {
                kept.insert(key.to_owned(), serde_json::json!({"id": given}));
            }
        }
        Value::Object(kept)
    } else {
        serde_json::json!({
            "code": value["error"]["code"],
            "message": held(&text_of(&value["error"]["message"]), 1024),
        })
    };
    DriveEvent::Reply { id, ok, result }
}
