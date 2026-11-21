//! One thread of the runner: what its program said, the requests that wait for the person, and
//! its state by the order of trust of THR-085 (the end of the process, then the stream of the
//! program). Pure: each call gives the [`Step`]s that the runner does, in order.

use std::collections::{BTreeMap, VecDeque};

use agent_threads::control::{Change, Refusal, RunState, ThreadRow, Update};
use agent_threads::drive::{
    ClaudeDecoder, ClaudeInput, DriveEvent, Effort, Program, ProgramState, Request,
};
use agent_threads::first_line;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::codex::Codex;

mod asks;
mod events;
mod kept;

/// How many updates of a thread the runner keeps for a client that starts to watch, and their
/// most bytes as JSON: a line of a program can be 4 MiB, so a count alone does not bound them.
const KEPT: usize = 4000;
const KEPT_BYTES: usize = 4 * 1024 * 1024;

/// The source of a prompt that came through the local socket (THR-042).
pub const THIS_COMPUTER: &str = "this computer";

/// Something the runner does for a thread, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// A line to the standard input of the program.
    Write(String),
    /// Keep the processes below the program now as spared (the MCP servers of Codex).
    Mark,
    /// End the processes below the program that are not spared (the stop of a Codex turn).
    EndChildren,
    /// End the program: it cannot go on (Codex did not start its thread).
    EndProgram,
    /// A line of the log of the runner, of the kind "stop", for a stop that the runner made.
    Log(&'static str),
    /// An update for the clients.
    Change(Change),
}

/// A request that waits for the person, with the line it came in, so that a runner that starts
/// again reads it again (THR-030).
#[derive(Clone, Debug)]
pub struct Open {
    pub request: Request,
    pub line: String,
    /// An answer was given and waits for the holder to take it; a second one is refused.
    pub answering: bool,
}

#[derive(Debug)]
enum Driver {
    Claude(ClaudeDecoder),
    Codex(Box<Codex>),
}

/// What the runner writes to its record for a thread.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Saved {
    pub row: ThreadRow,
    pub effort: Option<Effort>,
    pub codex_thread: Option<String>,
    pub codex_turn: Option<String>,
    /// The open requests: the ID of the runner and the line of the program.
    pub open: Vec<(Uuid, String)>,
    /// The open requests whose answer went to the holder: after a restart they close, so that
    /// the program never gets a second answer (THR-030).
    #[serde(default)]
    pub answering: Vec<Uuid>,
    /// The number of the last update, so that a client that watches across a restart of the
    /// runner never takes a new update for an old one.
    #[serde(default)]
    pub seq: u64,
}

/// One thread.
#[derive(Debug)]
pub struct Thread {
    pub row: ThreadRow,
    effort: Option<Effort>,
    driver: Driver,
    open: BTreeMap<Uuid, Open>,
    program: Option<ProgramState>,
    seq: u64,
    kept: VecDeque<(Update, usize)>,
    kept_bytes: usize,
    controls: u64,
    /// The holder started the program: a line may be written to it.
    held: bool,
    last_error: Option<String>,
}

impl Thread {
    #[must_use]
    pub fn new(row: ThreadRow, effort: Option<Effort>, codex: Option<Codex>) -> Self {
        let driver = match codex {
            Some(codex) => Driver::Codex(Box::new(codex)),
            None => Driver::Claude(ClaudeDecoder::new()),
        };
        Self {
            row,
            effort,
            driver,
            open: BTreeMap::new(),
            program: None,
            seq: 0,
            kept: VecDeque::new(),
            kept_bytes: 0,
            controls: 0,
            held: false,
            last_error: None,
        }
    }

    /// The record of the thread.
    #[must_use]
    pub fn saved(&self) -> Saved {
        let (codex_thread, codex_turn) = match &self.driver {
            Driver::Codex(codex) => (codex.thread.clone(), codex.turn.clone()),
            Driver::Claude(_) => (None, None),
        };
        Saved {
            row: self.row.clone(),
            effort: self.effort,
            codex_thread,
            codex_turn,
            open: self
                .open
                .iter()
                .map(|(id, open)| (*id, open.line.clone()))
                .collect(),
            seq: self.seq,
            answering: self
                .open
                .iter()
                .filter(|(_, open)| open.answering)
                .map(|(id, _)| *id)
                .collect(),
        }
    }

    /// What the record holds of this thread but the lines of its requests: when it does not
    /// change, the record need not be written again.
    #[must_use]
    pub fn record_key(&self) -> String {
        let (codex_thread, codex_turn) = match &self.driver {
            Driver::Codex(codex) => (codex.thread.as_deref(), codex.turn.as_deref()),
            Driver::Claude(_) => (None, None),
        };
        let open: Vec<(&Uuid, bool)> = self
            .open
            .iter()
            .map(|(id, open)| (id, open.answering))
            .collect();
        serde_json::to_string(&(&self.row, codex_thread, codex_turn, open)).unwrap_or_default()
    }

    /// A thread from its record, after a start of the runner; its open requests are read again
    /// from their lines and keep their IDs.
    #[must_use]
    pub fn restore(saved: Saved, codex: Option<Codex>) -> Self {
        let codex = codex.map(|codex| codex.known(saved.codex_thread, saved.codex_turn));
        let mut thread = Self::new(saved.row, saved.effort, codex);
        // What the program last said stays true until it says more (THR-085).
        thread.program = match thread.row.state {
            RunState::Idle => Some(ProgramState::Idle),
            RunState::Working | RunState::NeedsYou => Some(ProgramState::Working),
            RunState::Starting | RunState::Stopped | RunState::Unknown => None,
        };
        thread.held = thread.row.state != RunState::Stopped;
        thread.seq = saved.seq;
        for (id, line) in saved.open {
            let events = thread.decode(line.as_bytes());
            if let Some(DriveEvent::Request(request)) = events.into_iter().next() {
                thread.open.insert(
                    id,
                    Open {
                        request,
                        line,
                        answering: saved.answering.contains(&id),
                    },
                );
            }
        }
        thread
    }

    fn decode(&mut self, line: &[u8]) -> Vec<DriveEvent> {
        match &mut self.driver {
            Driver::Claude(decoder) => decoder.read(line),
            Driver::Codex(codex) => codex.decoder.read(line),
        }
    }

    /// The first steps of a new thread: its prompt, and the first line of the program.
    pub fn begin(&mut self, prompt: &str) -> Vec<Step> {
        self.held = true;
        self.row.title = first_line(prompt, agent_threads::TITLE_LENGTH);
        let mut steps = vec![prompt_change(prompt)];
        match &mut self.driver {
            Driver::Claude(_) => steps.push(Step::Write(ClaudeInput::prompt(prompt))),
            Driver::Codex(codex) => {
                steps.push(Step::Write(codex.begin()));
                codex.prompt(prompt);
            }
        }
        steps
    }

    /// The lines of a prompt of the person to a running thread. The prompt is shown as sent
    /// only when the holder took them ([`Self::sent`]); a Codex prompt before its thread has an
    /// ID waits in the thread and has no line yet.
    ///
    /// # Errors
    /// [`Refusal::NotRunning`] when the program ended or the holder did not start it yet.
    pub fn send(&mut self, text: &str) -> Result<Vec<String>, Refusal> {
        if self.row.state == RunState::Stopped || !self.held {
            return Err(Refusal::NotRunning);
        }
        Ok(match &mut self.driver {
            Driver::Claude(_) => vec![ClaudeInput::prompt(text)],
            Driver::Codex(codex) => codex.prompt(text).into_iter().collect(),
        })
    }

    /// The holder took the lines of a prompt: it is shown with its source (THR-042).
    #[must_use]
    pub fn sent(text: &str) -> Vec<Step> {
        vec![prompt_change(text)]
    }

    /// The person stops the running turn; the program stays. Each open request closes (THR-087:
    /// a stop by the person).
    pub fn interrupt(&mut self) -> Vec<Step> {
        let mut steps = self.turn_stop();
        steps.extend(self.close_all("the turn was stopped"));
        steps.extend(self.state_change());
        steps
    }

    /// The lines that stop the running turn. Codex (TH10) and Claude Code (seen on a Mac on
    /// 2026-11-21: a `sleep` of the Bash tool ran on) leave the command of a turn alive after
    /// their interrupt, so the processes below the program that the turn started end too
    /// (THR-049); its MCP servers, marked before the turn, are spared. With no running turn of
    /// Codex there is nothing to stop.
    fn turn_stop(&mut self) -> Vec<Step> {
        match &mut self.driver {
            Driver::Claude(_) => {
                self.controls += 1;
                let id = format!("anima-interrupt-{}", self.controls);
                vec![Step::Write(ClaudeInput::interrupt(&id)), Step::EndChildren]
            }
            Driver::Codex(codex) => codex
                .interrupt()
                .map_or_else(Vec::new, |line| vec![Step::Write(line), Step::EndChildren]),
        }
    }

    /// The program ended: each open request closes and the thread is stopped (THR-030, THR-085).
    pub fn exited(&mut self, code: Option<i32>, signal: Option<i32>) -> Vec<Step> {
        let mut steps = self.close_all("the process of the program ended");
        let why = match (code, signal) {
            (Some(code), _) if code != 0 => format!("The program ended with code {code}."),
            (None, Some(signal)) => format!("The program was ended by signal {signal}."),
            _ => "The program ended.".to_owned(),
        };
        let why = match &self.last_error {
            Some(error) if code != Some(0) => format!("{why} It said: {error}"),
            _ => why,
        };
        self.row.ended = Some(why);
        self.row.state = RunState::Stopped;
        self.row.open.clear();
        steps.push(Step::Change(Change::State {
            state: RunState::Stopped,
            row: self.row.clone(),
        }));
        steps
    }

    /// The thread was not running when the runner started: its holder ended (THR-049).
    pub fn lost(&mut self) -> Vec<Step> {
        let mut steps = self.exited(None, None);
        self.row.ended = Some("The holder of the program ended; the thread stopped.".to_owned());
        if let Some(Step::Change(Change::State { row, .. })) = steps.last_mut() {
            row.ended.clone_from(&self.row.ended);
        }
        steps
    }

    /// One line of standard error: kept as the last thing the program said of a fault.
    pub fn error_line(&mut self, text: &str) {
        let text = first_line(text, 300);
        if !text.is_empty() {
            self.last_error = Some(text);
        }
    }

    /// The state now: an open request first (sticky, THR-086), then what the program said.
    fn state(&self) -> RunState {
        if self.row.state == RunState::Stopped {
            return RunState::Stopped;
        }
        if !self.open.is_empty() {
            return RunState::NeedsYou;
        }
        match self.program {
            None => RunState::Starting,
            Some(ProgramState::Working | ProgramState::NeedsAnswer) => RunState::Working,
            Some(ProgramState::Idle) => RunState::Idle,
        }
    }

    /// A change of the state, when the state or the open requests changed.
    fn state_change(&mut self) -> Option<Step> {
        let state = self.state();
        let open: Vec<Uuid> = self.open.keys().copied().collect();
        if state == self.row.state && open == self.row.open {
            return None;
        }
        self.row.state = state;
        self.row.open = open;
        Some(Step::Change(Change::State {
            state,
            row: self.row.clone(),
        }))
    }

    /// The program of the thread.
    #[must_use]
    pub const fn program(&self) -> Program {
        self.row.program
    }
}

fn said(event: DriveEvent) -> Step {
    Step::Change(Change::Said { event })
}

fn closed(request: Uuid, why: &str) -> Step {
    Step::Change(Change::Closed {
        request,
        why: why.to_owned(),
    })
}

fn prompt_change(text: &str) -> Step {
    Step::Change(Change::Prompt {
        text: text.to_owned(),
        source: THIS_COMPUTER.to_owned(),
    })
}
