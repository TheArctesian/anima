//! One session of Claude Code or Codex read as a thread (THR-003 to THR-006 of
//! `docs/proposals/agent-threads.md`): the prompts, the replies, the reasoning, the tool calls
//! with their results and patches, and the ends of the turns, in the order of the file.
//!
//! A crate of its own, so that the `anima` command is built from this reader alone and not from
//! the core of the daemon (`deploy/release/applications.txt`).
//!
//! Pure: [`ThreadReader`] takes the lines of one session file and gives a [`Thread`]. It reads
//! the file of the program as the program wrote it and starts nothing. The text it keeps is
//! held to [`MAX_TEXT`] for each entry and [`MAX_ENTRIES`] for each thread. Whoever reads the
//! files (I/O) gives the lines; nothing here logs.

mod claude;
mod codex;
pub mod control;
pub mod drive;
#[cfg(test)]
mod tests;
mod tool;

pub use tool::ToolKind;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

/// The agent program a session file comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    ClaudeCode,
    Codex,
}

/// A time of a line.
type Time = DateTime<Utc>;

/// The most bytes of text one entry keeps; the rest is cut at a character boundary.
pub const MAX_TEXT: usize = 64 * 1024;

/// The most bytes of the output of one tool call.
pub const MAX_OUTPUT: usize = 16 * 1024;

/// The most entries one thread keeps: the later ones are counted in [`Thread::dropped`].
pub const MAX_ENTRIES: usize = 20_000;

/// The most bytes of text one thread keeps in its entries: the later entries are counted in
/// [`Thread::dropped`], so a file of any size gives a thread of a known size.
pub const MAX_THREAD: usize = 8 * 1024 * 1024;

/// The most characters of a title.
pub const TITLE_LENGTH: usize = 80;

/// The most characters of a folder, a branch, a model or an ID, each one line.
pub const NAME_LENGTH: usize = 1024;

/// One session of an agent program, read from its file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Thread {
    /// The ID of the session: the file name of Claude Code, the `id` of a Codex rollout.
    pub id: Option<String>,
    /// The title the program or the person gave it, else the start of the first prompt.
    pub title: Option<String>,
    /// The program or the person gave the title; it is not the start of a prompt.
    pub title_given: bool,
    /// The working folder of the session.
    pub folder: Option<String>,
    /// The git branch of the working folder, as the program wrote it.
    pub branch: Option<String>,
    /// The last model named.
    pub model: Option<String>,
    pub started: Option<DateTime<Utc>>,
    pub last: Option<DateTime<Utc>>,
    /// How the file ends: what the last lines say of the turn.
    pub tail: Tail,
    /// The prompts the person typed.
    pub prompts: u64,
    pub entries: Vec<Entry>,
    /// Entries past [`MAX_ENTRIES`], not kept.
    pub dropped: u64,
}

/// How a session file ends (THR-055). It is what the file says, not a state of the program: a
/// file cannot tell whether an open tool call runs or waits for an approval.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tail {
    /// No prompt, reply or tool call was read.
    #[default]
    Nothing,
    /// The agent ended its turn: the next step is a prompt of the person.
    TurnEnded,
    /// The person or the program stopped the turn.
    Stopped,
    /// A tool call has no result yet: it runs, or it waits for an approval.
    OpenTool,
    /// The turn is open: a prompt, a reply or a result is the last line.
    Open,
}

impl Tail {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nothing => "nothing",
            Self::TurnEnded => "turn_ended",
            Self::Stopped => "stopped",
            Self::OpenTool => "open_tool",
            Self::Open => "open",
        }
    }
}

/// One step of a thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub at: Option<DateTime<Utc>>,
    pub kind: EntryKind,
}

/// What one step is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// Text the person typed, and the number of images that came with it.
    Prompt { text: String, images: u32 },
    /// Text of the agent.
    Reply { text: String },
    /// The reasoning, or its summary, as the program wrote it.
    Reasoning { text: String },
    /// A call of a tool.
    Tool(ToolCall),
    /// The end of a turn; `stopped` when the person or the program stopped it.
    TurnEnd { stopped: bool },
    /// The program made the earlier part of the thread shorter.
    Compacted,
}

/// A call of a tool and, when it came, its result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub call: String,
    pub name: String,
    pub kind: ToolKind,
    /// One line that says what the call does: a command, a path, a pattern.
    pub summary: String,
    /// The input as the program wrote it.
    pub input: String,
    /// The change of a file as lines that start with `-`, `+` or a space; for an edit only.
    pub patch: Option<String>,
    pub output: Option<String>,
    pub failed: bool,
}

/// Reads the lines of one session file, in order, into a [`Thread`].
#[derive(Debug)]
pub struct ThreadReader {
    source: Source,
    thread: Thread,
    /// The entry of each tool call, by the ID of the call, to attach its result.
    calls: HashMap<String, usize>,
    /// A title the program or the person gave, which wins over the first prompt.
    named: Option<String>,
    /// The bytes of text the entries hold.
    kept: usize,
    /// The tool calls that have no result yet, by the ID of the call.
    open: HashSet<String>,
}

impl ThreadReader {
    #[must_use]
    pub fn new(source: Source) -> Self {
        Self {
            source,
            thread: Thread::default(),
            calls: HashMap::new(),
            named: None,
            kept: 0,
            open: HashSet::new(),
        }
    }

    /// Reads one line (without its newline). A line that is not JSON, or that this reader does
    /// not know, changes nothing.
    pub fn feed(&mut self, line: &[u8]) {
        match self.source {
            Source::ClaudeCode => claude::read(self, line),
            Source::Codex => codex::read(self, line),
        }
    }

    /// The thread read so far.
    #[must_use]
    pub fn finish(mut self) -> Thread {
        if let Some(named) = self.named.take() {
            self.thread.title = Some(named);
            self.thread.title_given = true;
        }
        self.thread
    }

    fn at(&mut self, at: Option<DateTime<Utc>>) {
        if let Some(at) = at {
            self.thread.started.get_or_insert(at);
            if self.thread.last.is_none_or(|last| last < at) {
                self.thread.last = Some(at);
            }
        }
    }

    fn folder(&mut self, folder: Option<&str>, branch: Option<&str>) {
        if let Some(folder) = folder.map(name).filter(|folder| !folder.is_empty()) {
            self.thread.folder = Some(folder);
        }
        if let Some(branch) = branch.map(name).filter(|branch| !branch.is_empty()) {
            self.thread.branch = Some(branch);
        }
    }

    fn model(&mut self, model: Option<&str>) {
        if let Some(model) = model.filter(|model| !model.is_empty() && !model.starts_with('<')) {
            self.thread.model = Some(name(model));
        }
    }

    fn name(&mut self, title: &str) {
        let title = first_line(title, TITLE_LENGTH);
        if !title.is_empty() {
            self.named = Some(title);
        }
    }

    fn prompt(&mut self, at: Option<DateTime<Utc>>, text: &str, images: u32) {
        if text.trim().is_empty() && images == 0 {
            return;
        }
        self.thread.prompts += 1;
        self.open.clear();
        self.thread.tail = Tail::Open;
        if self.thread.title.is_none() {
            let title = first_line(text, TITLE_LENGTH);
            self.thread.title = (!title.is_empty()).then_some(title);
        }
        self.push(
            at,
            EntryKind::Prompt {
                text: held(text, MAX_TEXT),
                images,
            },
        );
    }

    fn text(&mut self, at: Option<DateTime<Utc>>, reasoning: bool, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let text = held(text, MAX_TEXT);
        if self.open.is_empty() {
            self.thread.tail = Tail::Open;
        }
        self.push(
            at,
            if reasoning {
                EntryKind::Reasoning { text }
            } else {
                EntryKind::Reply { text }
            },
        );
    }

    fn call(&mut self, at: Option<DateTime<Utc>>, call: ToolCall) {
        let id = call.call.clone();
        self.open.insert(id.clone());
        self.thread.tail = Tail::OpenTool;
        if self.push(at, EntryKind::Tool(call)) && !id.is_empty() {
            self.calls.insert(id, self.thread.entries.len() - 1);
        }
    }

    /// Attaches a result to the call `call`; a result with no known call is not kept.
    fn result(&mut self, call: &str, output: &str, failed: bool) {
        if self.open.remove(call) && self.open.is_empty() {
            self.thread.tail = Tail::Open;
        }
        let Some(&index) = self.calls.get(call) else {
            return;
        };
        if let Some(Entry {
            kind: EntryKind::Tool(tool),
            ..
        }) = self.thread.entries.get_mut(index)
        {
            tool.output = Some(held(output, MAX_OUTPUT));
            tool.failed = failed;
        }
    }

    /// The end of a turn; `stopped` when the person or the program stopped it.
    fn end(&mut self, at: Option<DateTime<Utc>>, stopped: bool) {
        self.open.clear();
        self.thread.tail = if stopped {
            Tail::Stopped
        } else {
            Tail::TurnEnded
        };
        self.push(at, EntryKind::TurnEnd { stopped });
    }

    fn push(&mut self, at: Option<DateTime<Utc>>, kind: EntryKind) -> bool {
        self.at(at);
        self.kept = self.kept.saturating_add(size(&kind));
        if self.thread.entries.len() >= MAX_ENTRIES || self.kept > MAX_THREAD {
            self.thread.dropped += 1;
            return false;
        }
        self.thread.entries.push(Entry { at, kind });
        true
    }
}

/// The bytes of text an entry holds.
fn size(kind: &EntryKind) -> usize {
    match kind {
        EntryKind::Prompt { text, .. }
        | EntryKind::Reply { text }
        | EntryKind::Reasoning { text } => text.len(),
        EntryKind::Tool(call) => {
            call.input.len() + call.patch.as_ref().map_or(0, String::len) + MAX_OUTPUT
        }
        EntryKind::TurnEnd { .. } | EntryKind::Compacted => 0,
    }
}

/// A folder, a branch, a model or an ID as one line of at most [`NAME_LENGTH`] characters.
fn name(text: &str) -> String {
    first_line(text, NAME_LENGTH)
}

/// The first line of `text` that is not blank, at most `length` characters, trimmed.
#[must_use]
pub fn first_line(text: &str, length: usize) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    line.chars().take(length).collect()
}

/// `text`, cut to at most `limit` bytes at a character boundary, with a mark when cut.
#[must_use]
pub fn held(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n[cut: {} more bytes]", &text[..end], text.len() - end)
}

fn time(text: Option<&str>) -> Option<DateTime<Utc>> {
    Some(
        DateTime::parse_from_rfc3339(text?)
            .ok()?
            .with_timezone(&Utc),
    )
}
