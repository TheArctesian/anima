//! The JSON-RPC lines that the runner writes to `codex app-server` (TH10 Section 3.2, TH18
//! Section 3). Only the methods of [`CodexMethod`] can be written: there is no way to send a
//! method by its name (THR-021).

use std::path::Path;

use serde_json::{Value, json};

use super::DriveError;
use super::launch::{Effort, Mode, Model};
use super::request::{Answer, ProgramRequestId, Request, RequestKind};

/// The closed list of the Codex methods that Anima may call (THR-021, R-290).
/// Not in it until OD-THR40 is answered: `chatgptAuthTokens` and each method that takes or
/// gives a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexMethod {
    Initialize,
    Initialized,
    ThreadStart,
    ThreadResume,
    ThreadFork,
    TurnStart,
    TurnSteer,
    TurnInterrupt,
    AccountRead,
    AccountRateLimitsRead,
    /// Only when the person asks for the sign-in at the client of the same computer.
    AccountLoginStart,
}

impl CodexMethod {
    pub const ALL: [Self; 11] = [
        Self::Initialize,
        Self::Initialized,
        Self::ThreadStart,
        Self::ThreadResume,
        Self::ThreadFork,
        Self::TurnStart,
        Self::TurnSteer,
        Self::TurnInterrupt,
        Self::AccountRead,
        Self::AccountRateLimitsRead,
        Self::AccountLoginStart,
    ];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Initialized => "initialized",
            Self::ThreadStart => "thread/start",
            Self::ThreadResume => "thread/resume",
            Self::ThreadFork => "thread/fork",
            Self::TurnStart => "turn/start",
            Self::TurnSteer => "turn/steer",
            Self::TurnInterrupt => "turn/interrupt",
            Self::AccountRead => "account/read",
            Self::AccountRateLimitsRead => "account/rateLimits/read",
            Self::AccountLoginStart => "account/login/start",
        }
    }
}

/// A turn of a Codex thread: the IDs that the server gave.
#[derive(Clone, Copy, Debug)]
pub struct CodexTurn<'a> {
    pub thread: &'a str,
    pub turn: &'a str,
}

/// The writer of the input lines of `codex app-server`. Each call takes the JSON-RPC ID that
/// the runner chose.
pub struct CodexInput;

impl CodexInput {
    #[must_use]
    pub fn initialize(id: i64) -> String {
        call(
            id,
            CodexMethod::Initialize,
            &json!({"clientInfo": {"name": "anima", "version": env!("CARGO_PKG_VERSION")}}),
        )
    }

    /// The notice that follows the answer to `initialize`.
    #[must_use]
    pub fn initialized() -> String {
        json!({"jsonrpc": "2.0", "method": CodexMethod::Initialized.name()}).to_string()
    }

    /// A new thread in `folder`; the mode goes here, so the project file cannot change it
    /// (TH18 3.3).
    #[must_use]
    pub fn thread_start(id: i64, folder: &Path, mode: Mode, model: Option<&Model>) -> String {
        call(
            id,
            CodexMethod::ThreadStart,
            &thread_params(None, folder, mode, model),
        )
    }

    #[must_use]
    pub fn thread_resume(id: i64, thread: &str, folder: &Path, mode: Mode) -> String {
        call(
            id,
            CodexMethod::ThreadResume,
            &thread_params(Some(thread), folder, mode, None),
        )
    }

    #[must_use]
    pub fn thread_fork(id: i64, thread: &str, folder: &Path, mode: Mode) -> String {
        call(
            id,
            CodexMethod::ThreadFork,
            &thread_params(Some(thread), folder, mode, None),
        )
    }

    #[must_use]
    pub fn turn_start(id: i64, thread: &str, text: &str, effort: Option<Effort>) -> String {
        let mut params = json!({"threadId": thread, "input": [{"type": "text", "text": text}]});
        if let Some(effort) = effort {
            params["effort"] = Value::from(effort.as_str());
        }
        call(id, CodexMethod::TurnStart, &params)
    }

    /// A message into the running turn.
    #[must_use]
    pub fn turn_steer(id: i64, turn: CodexTurn<'_>, text: &str) -> String {
        let params = json!({
            "threadId": turn.thread,
            "expectedTurnId": turn.turn,
            "input": [{"type": "text", "text": text}],
        });
        call(id, CodexMethod::TurnSteer, &params)
    }

    /// Ends the turn for the server. It did not end the running command in TH10: the runner
    /// ends the process tree itself (THR-049).
    #[must_use]
    pub fn turn_interrupt(id: i64, turn: CodexTurn<'_>) -> String {
        call(
            id,
            CodexMethod::TurnInterrupt,
            &json!({"threadId": turn.thread, "turnId": turn.turn}),
        )
    }

    /// The account and the plan; the tokens stay in the store of Codex (THR-115).
    #[must_use]
    pub fn account_read(id: i64) -> String {
        call(id, CodexMethod::AccountRead, &json!({}))
    }

    /// The limits of the account and when they reset (THR-115).
    #[must_use]
    pub fn account_limits_read(id: i64) -> String {
        call(id, CodexMethod::AccountRateLimitsRead, &json!(null))
    }

    /// The reply to a server request. "Allow for the session" sends back one decision that the
    /// request listed (THR-030); a deny is `decline`, so the turn goes on.
    ///
    /// # Errors
    /// [`DriveError::AnswerDoesNotFit`] when the answer does not fit the request.
    pub fn answer(request: &Request, answer: &Answer) -> Result<String, DriveError> {
        answer.check(request)?;
        let result = match answer {
            Answer::Allow => json!({"decision": "accept"}),
            Answer::AllowForSession { rule } => json!({"decision": request.offered[*rule]}),
            Answer::Deny { .. } if request.kind == RequestKind::Question => json!({"answers": {}}),
            Answer::Deny { .. } => json!({"decision": "decline"}),
            Answer::Choose { answers } => {
                let mut grouped = serde_json::Map::new();
                for (key, choice) in answers {
                    let entry = grouped
                        .entry(key.clone())
                        .or_insert_with(|| json!({"answers": []}));
                    if let Some(list) = entry["answers"].as_array_mut() {
                        list.push(Value::from(choice.as_str()));
                    }
                }
                json!({"answers": grouped})
            }
        };
        Ok(json!({"jsonrpc": "2.0", "id": request.id.json(), "result": result}).to_string())
    }

    /// The refusal of a server request of a kind the runner does not know (THR-046).
    #[must_use]
    pub fn refuse(id: &ProgramRequestId, why: &str) -> String {
        json!({"jsonrpc": "2.0", "id": id.json(), "error": {"code": -32601, "message": why}})
            .to_string()
    }
}

fn call(id: i64, method: CodexMethod, params: &Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "method": method.name(), "params": params}).to_string()
}

fn thread_params(thread: Option<&str>, folder: &Path, mode: Mode, model: Option<&Model>) -> Value {
    let (sandbox, approval) = mode.codex();
    let mut params = json!({
        "cwd": folder.to_string_lossy(),
        "sandbox": sandbox,
        "approvalPolicy": approval,
        "approvalsReviewer": "user",
    });
    if let Some(thread) = thread {
        params["threadId"] = Value::from(thread);
        params["excludeTurns"] = Value::from(true);
    }
    if let Some(model) = model {
        params["model"] = Value::from(model.as_str());
    }
    params
}
