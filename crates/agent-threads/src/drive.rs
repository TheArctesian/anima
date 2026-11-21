//! The protocols by which the thread runner drives Claude Code and Codex (THR-020, THR-021 of
//! `docs/proposals/agent-threads.md`; measured in TH10 and TH18).
//!
//! Pure: this module builds the command line of a program, writes the lines that go to its
//! standard input, and reads the lines of its standard output as [`DriveEvent`]s. It starts no
//! process and reads no file; the runner of the daemon does the I/O.
//!
//! What it refuses, so that the runner cannot do it by mistake:
//! - a value of an argument that is not of its closed alphabet, or that starts with `-` (THR-040);
//! - a Codex method that is not in [`CodexMethod`] (THR-021);
//! - an answer that names a rule the request did not offer (THR-030);
//! - an event or a request of a kind it does not know: it gives [`DriveEvent::Unknown`], which
//!   stops the turn when it could wait for an answer (THR-046).

mod claude_in;
mod claude_out;
mod codex_in;
mod codex_out;
mod launch;
mod request;
#[cfg(test)]
mod tests;

pub use claude_in::ClaudeInput;
pub use claude_out::ClaudeDecoder;
pub use codex_in::{CodexInput, CodexMethod, CodexTurn};
pub use codex_out::CodexDecoder;
pub use launch::{Effort, Invocation, Launch, Mode, Model, Program, SessionStart};
pub use request::{Answer, ProgramRequestId, Question, Request, RequestKind};

use serde::Serialize;

/// The most bytes of one line of a program that a decoder reads; a longer line is
/// [`DriveEvent::Unknown`] and stops the turn.
pub const MAX_LINE: usize = 16 * 1024 * 1024;

/// What the program says it does now (THR-085: the structured stream is the second source of
/// trust, after the end of the process).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramState {
    Working,
    /// The program waits for an answer to a request.
    NeedsAnswer,
    Idle,
}

/// How a turn ended, from the end event of the program, never from its exit code (TH10: the
/// exit code of `claude` is 0 after an interrupt).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnOutcome {
    Completed,
    Interrupted,
    Failed,
}

/// One thing a program said, in the order it said it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DriveEvent {
    /// The program named the session (Claude Code) or the thread (Codex). It comes again when
    /// the ID changes, for example after a compaction (THR-089).
    Session {
        id: String,
        model: Option<String>,
        mode: Option<String>,
    },
    /// A turn started; Codex names it, and an interrupt or a steer needs the name.
    TurnStarted {
        turn: Option<String>,
    },
    State {
        state: ProgramState,
    },
    /// A piece of the reply.
    TextDelta {
        text: String,
    },
    /// A piece of the reasoning.
    ReasoningDelta {
        text: String,
    },
    /// A whole reply. `parent` is the tool call of a sub agent that said it (THR-089).
    Message {
        text: String,
        parent: Option<String>,
    },
    ToolCall {
        id: String,
        name: String,
        input: String,
        parent: Option<String>,
    },
    /// A piece of the output of a running tool call (Codex).
    ToolOutputDelta {
        id: String,
        text: String,
    },
    ToolOutput {
        id: String,
        output: String,
        failed: bool,
        decided_by: Option<String>,
    },
    /// The program asks the person (THR-030, THR-117).
    Request(Request),
    /// The program says that a request is answered or gone.
    RequestClosed {
        id: ProgramRequestId,
    },
    /// The program answered a request of the runner (`initialize`, `interrupt`, a JSON-RPC call).
    Reply {
        id: ProgramRequestId,
        ok: bool,
        result: serde_json::Value,
    },
    TurnEnded {
        outcome: TurnOutcome,
        detail: Option<String>,
        refused: u32,
    },
    /// A notice of the program for the person: a configuration warning, an unknown system kind.
    Notice {
        text: String,
    },
    /// A line of a kind this module does not know. `stops_turn` is true when the line could wait
    /// for an answer or is not a line of the protocol at all (THR-046: fail closed).
    Unknown {
        what: String,
        stops_turn: bool,
    },
}

/// A fault of a value given to this module.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DriveError {
    #[error("the value of {what} is not permitted")]
    Refused { what: &'static str },
    #[error("the program path must be absolute")]
    RelativeProgram,
    #[error("the answer does not fit the request: {0}")]
    AnswerDoesNotFit(&'static str),
}

/// The text of a JSON value for the view: a string as it is, anything else as compact JSON.
fn text_of(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
