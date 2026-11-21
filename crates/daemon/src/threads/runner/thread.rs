//! One thread of the runner: what its program said, the requests that wait for the person, and
//! its state by the order of trust of THR-085 (the end of the process, then the stream of the
//! program). Pure: each call gives the [`Step`]s that the runner does, in order.

use std::collections::{BTreeMap, VecDeque};

use agent_threads::control::{Change, Refusal, RunState, ThreadRow, Update};
use agent_threads::drive::{
    Answer, ClaudeDecoder, ClaudeInput, DriveEvent, Effort, Program, ProgramState, Request,
};
use agent_threads::first_line;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::codex::Codex;

mod events;

/// How many updates of a thread the runner keeps for a client that starts to watch.
const KEPT: usize = 4000;

/// The source of a prompt that came through the local socket (THR-042).
pub const THIS_COMPUTER: &str = "this computer";

/// Something the runner does for a thread, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// A line to the standard input of the program.
    Write(String),
    /// End the processes below the program, not the program (the stop of a Codex turn).
    EndChildren,
    /// An update for the clients.
    Change(Change),
}

/// A request that waits for the person, with the line it came in, so that a runner that starts
/// again reads it again (THR-030).
#[derive(Clone, Debug)]
pub struct Open {
    pub request: Request,
    pub line: String,
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
    kept: VecDeque<Update>,
    controls: u64,
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
            controls: 0,
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
        }
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
        for (id, line) in saved.open {
            let events = thread.decode(line.as_bytes());
            if let Some(DriveEvent::Request(request)) = events.into_iter().next() {
                thread.open.insert(id, Open { request, line });
            }
        }
        thread
    }

    /// The updates the runner keeps, for a client that starts to watch.
    pub fn kept(&self) -> impl Iterator<Item = &Update> {
        self.kept.iter()
    }

    /// Gives `change` the next number of the thread and keeps it.
    pub fn note(&mut self, change: Change) -> Update {
        self.seq += 1;
        let update = Update {
            thread: self.row.id,
            seq: self.seq,
            change,
        };
        self.kept.push_back(update.clone());
        while self.kept.len() > KEPT {
            self.kept.pop_front();
        }
        update
    }

    fn decode(&mut self, line: &[u8]) -> Vec<DriveEvent> {
        match &mut self.driver {
            Driver::Claude(decoder) => decoder.read(line),
            Driver::Codex(codex) => codex.decoder.read(line),
        }
    }

    /// The first steps of a new thread: its prompt, and the first line of the program.
    pub fn begin(&mut self, prompt: &str) -> Vec<Step> {
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

    /// A prompt of the person to a running thread.
    ///
    /// # Errors
    /// [`Refusal::NotRunning`] when the program ended.
    pub fn send(&mut self, text: &str) -> Result<Vec<Step>, Refusal> {
        if self.row.state == RunState::Stopped {
            return Err(Refusal::NotRunning);
        }
        let mut steps = vec![prompt_change(text)];
        match &mut self.driver {
            Driver::Claude(_) => steps.push(Step::Write(ClaudeInput::prompt(text))),
            Driver::Codex(codex) => steps.extend(codex.prompt(text).into_iter().map(Step::Write)),
        }
        Ok(steps)
    }

    /// The answer of the person to the request `id` of the runner. One answer for each request,
    /// while its process lives (THR-030).
    ///
    /// # Errors
    /// [`Refusal::RequestGone`] for a request that is answered, closed or of an ended process;
    /// [`Refusal::AnswerDoesNotFit`] for an answer of the wrong kind.
    pub fn answer(&mut self, id: Uuid, answer: &Answer) -> Result<Vec<Step>, Refusal> {
        if self.row.state == RunState::Stopped {
            return Err(Refusal::RequestGone);
        }
        let open = self.open.get(&id).ok_or(Refusal::RequestGone)?;
        let line = match &self.driver {
            Driver::Claude(_) => ClaudeInput::answer(&open.request, answer),
            Driver::Codex(_) => agent_threads::drive::CodexInput::answer(&open.request, answer),
        }
        .map_err(|_| Refusal::AnswerDoesNotFit)?;
        self.open.remove(&id);
        let mut steps = vec![Step::Write(line), closed(id, "answered by the person")];
        steps.extend(self.state_change());
        Ok(steps)
    }

    /// Stops the running turn; the program stays. Each open request closes (THR-087: a stop by
    /// the person). Codex leaves its command alive after `turn/interrupt` (TH10), so the
    /// processes below it end too (THR-049).
    pub fn interrupt(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        match &mut self.driver {
            Driver::Claude(_) => {
                self.controls += 1;
                let id = format!("anima-interrupt-{}", self.controls);
                steps.push(Step::Write(ClaudeInput::interrupt(&id)));
            }
            Driver::Codex(codex) => {
                steps.extend(codex.interrupt().map(Step::Write));
                steps.push(Step::EndChildren);
            }
        }
        steps.extend(self.close_all("the turn was stopped"));
        steps.extend(self.state_change());
        steps
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

    fn close_all(&mut self, why: &str) -> Vec<Step> {
        std::mem::take(&mut self.open)
            .into_keys()
            .map(|id| closed(id, why))
            .collect()
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
