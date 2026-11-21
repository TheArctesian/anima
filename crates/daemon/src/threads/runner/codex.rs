//! The calls of the runner to one `codex app-server` (THR-021, TH10 Section 3): the start of the
//! server and of its thread, then a turn for each prompt. Pure: it gives the lines to write.

use std::collections::BTreeMap;
use std::path::PathBuf;

use agent_threads::drive::{CodexDecoder, CodexInput, CodexTurn, Effort, Mode, Model};
use serde_json::Value;

/// What a call of the runner was, so that its answer can be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    Initialize,
    ThreadStart,
    TurnStart,
    Other,
}

/// The state of the calls to one Codex server.
#[derive(Debug)]
pub struct Codex {
    pub decoder: CodexDecoder,
    folder: PathBuf,
    mode: Mode,
    model: Option<Model>,
    effort: Option<Effort>,
    next: i64,
    calls: BTreeMap<i64, Call>,
    /// The ID of the thread of Codex, from its answer to `thread/start`.
    pub thread: Option<String>,
    /// The ID of the running turn, while one runs.
    pub turn: Option<String>,
    /// The turn that ended last, so that a late answer to `turn/start` does not open it again.
    ended: Option<String>,
    /// Prompts that wait for the thread of Codex.
    queued: Vec<String>,
}

impl Codex {
    #[must_use]
    pub fn new(folder: PathBuf, mode: Mode, model: Option<Model>, effort: Option<Effort>) -> Self {
        Self {
            decoder: CodexDecoder::new(),
            folder,
            mode,
            model,
            effort,
            next: 1,
            calls: BTreeMap::new(),
            thread: None,
            turn: None,
            ended: None,
            queued: Vec::new(),
        }
    }

    /// A server that the holder kept while the daemon started again: its thread is known.
    #[must_use]
    pub fn known(mut self, thread: Option<String>, turn: Option<String>) -> Self {
        self.thread = thread;
        self.turn = turn;
        self.next = 1_000_000;
        self
    }

    fn call(&mut self, call: Call) -> i64 {
        let id = self.next;
        self.next += 1;
        self.calls.insert(id, call);
        id
    }

    /// The first line: `initialize`.
    pub fn begin(&mut self) -> String {
        let id = self.call(Call::Initialize);
        CodexInput::initialize(id)
    }

    /// A prompt: queued until the thread is there, a steer of the running turn, else a turn.
    pub fn prompt(&mut self, text: &str) -> Vec<String> {
        let Some(thread) = self.thread.clone() else {
            self.queued.push(text.to_owned());
            return Vec::new();
        };
        let id = self.call(Call::Other);
        if let Some(turn) = self.turn.clone() {
            return vec![CodexInput::turn_steer(
                id,
                CodexTurn {
                    thread: &thread,
                    turn: &turn,
                },
                text,
            )];
        }
        self.calls.insert(id, Call::TurnStart);
        vec![CodexInput::turn_start(id, &thread, text, self.effort)]
    }

    /// A turn started; Codex names it.
    pub fn turn_started(&mut self, turn: Option<String>) {
        if turn.is_some() {
            self.turn = turn;
        }
    }

    /// The running turn ended.
    pub fn turn_ended(&mut self) {
        self.ended = self.turn.take();
    }

    /// `turn/interrupt` for the running turn, if one runs.
    pub fn interrupt(&mut self) -> Option<String> {
        let thread = self.thread.clone()?;
        let turn = self.turn.clone()?;
        let id = self.call(Call::Other);
        Some(CodexInput::turn_interrupt(
            id,
            CodexTurn {
                thread: &thread,
                turn: &turn,
            },
        ))
    }

    /// The answer to a call of the runner: the lines that follow it, or the text of a fault
    /// that ends the thread.
    ///
    /// # Errors
    /// The text of the fault when the server or its thread did not start.
    pub fn reply(&mut self, id: i64, ok: bool, result: &Value) -> Result<Vec<String>, String> {
        let call = self.calls.remove(&id).unwrap_or(Call::Other);
        if !ok {
            let message = result["message"].as_str().unwrap_or("no reason given");
            return match call {
                Call::Initialize | Call::ThreadStart => {
                    Err(format!("Codex did not start its thread: {message}"))
                }
                Call::TurnStart | Call::Other => Ok(Vec::new()),
            };
        }
        match call {
            Call::Initialize => {
                let id = self.call(Call::ThreadStart);
                Ok(vec![
                    CodexInput::initialized(),
                    CodexInput::thread_start(id, &self.folder, self.mode, self.model.as_ref()),
                ])
            }
            Call::ThreadStart => {
                self.thread = result["thread"]["id"].as_str().map(str::to_owned);
                let queued = std::mem::take(&mut self.queued);
                Ok(queued.iter().flat_map(|text| self.prompt(text)).collect())
            }
            Call::TurnStart => {
                if let Some(turn) = result["turn"]["id"].as_str()
                    && self.ended.as_deref() != Some(turn)
                {
                    self.turn.get_or_insert_with(|| turn.to_owned());
                }
                Ok(Vec::new())
            }
            Call::Other => Ok(Vec::new()),
        }
    }
}
