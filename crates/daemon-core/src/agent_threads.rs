//! One session of Claude Code or Codex read as a thread (THR-001 to THR-006 of
//! `docs/proposals/agent-threads.md`): the prompts, the replies, the reasoning, the tool calls
//! with their results and patches, and the ends of the turns, in the order of the file.
//!
//! Pure: [`ThreadReader`] takes the lines of one session file and gives a [`Thread`]. It reads
//! the file of the program as the program wrote it and starts nothing. The text it keeps is
//! held to [`MAX_TEXT`] for each entry and [`MAX_ENTRIES`] for each thread. Whoever reads the
//! files (I/O) gives the lines; nothing here logs.

mod claude;
mod codex;
#[cfg(test)]
mod tests;
mod tool;

pub use tool::ToolKind;

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::agent_usage::Source;

/// A time of a line.
type Time = DateTime<Utc>;

/// The most bytes of text one entry keeps; the rest is cut at a character boundary.
pub const MAX_TEXT: usize = 64 * 1024;

/// The most bytes of the output of one tool call.
pub const MAX_OUTPUT: usize = 16 * 1024;

/// The most entries one thread keeps: the later ones are counted in [`Thread::dropped`].
pub const MAX_ENTRIES: usize = 20_000;

/// The most characters of a title made from the first prompt.
pub const TITLE_LENGTH: usize = 80;

/// One session of an agent program, read from its file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Thread {
    /// The ID of the session: the file name of Claude Code, the `id` of a Codex rollout.
    pub id: Option<String>,
    /// The title the program or the person gave it, else the start of the first prompt.
    pub title: Option<String>,
    /// The working folder of the session.
    pub folder: Option<String>,
    /// The git branch of the working folder, as the program wrote it.
    pub branch: Option<String>,
    /// The last model named.
    pub model: Option<String>,
    pub started: Option<DateTime<Utc>>,
    pub last: Option<DateTime<Utc>>,
    /// The prompts the person typed.
    pub prompts: u64,
    pub entries: Vec<Entry>,
    /// Entries past [`MAX_ENTRIES`], not kept.
    pub dropped: u64,
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
}

impl ThreadReader {
    #[must_use]
    pub fn new(source: Source) -> Self {
        Self {
            source,
            thread: Thread::default(),
            calls: HashMap::new(),
            named: None,
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
        if let Some(folder) = folder.filter(|folder| !folder.is_empty()) {
            self.thread.folder = Some(folder.to_owned());
        }
        if let Some(branch) = branch.filter(|branch| !branch.is_empty()) {
            self.thread.branch = Some(branch.to_owned());
        }
    }

    fn model(&mut self, model: Option<&str>) {
        if let Some(model) = model.filter(|model| !model.is_empty() && !model.starts_with('<')) {
            self.thread.model = Some(model.to_owned());
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
        if self.push(at, EntryKind::Tool(call)) && !id.is_empty() {
            self.calls.insert(id, self.thread.entries.len() - 1);
        }
    }

    /// Attaches a result to the call `call`; a result with no known call is not kept.
    fn result(&mut self, call: &str, output: &str, failed: bool) {
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

    fn push(&mut self, at: Option<DateTime<Utc>>, kind: EntryKind) -> bool {
        self.at(at);
        if self.thread.entries.len() >= MAX_ENTRIES {
            self.thread.dropped += 1;
            return false;
        }
        self.thread.entries.push(Entry { at, kind });
        true
    }
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
