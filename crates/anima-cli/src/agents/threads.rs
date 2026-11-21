//! `anima agents threads` and `anima agents thread`: the sessions of Claude Code and Codex on
//! this computer, read from their files as threads (THR-003 to THR-009). The command reads
//! files of this user only; it sends nothing to the hub and starts no program.

pub mod files;
pub mod shown;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use agent_threads::Source;
use agent_threads::Thread;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::environment::process_home;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::{Cell, Table, Tone};
use files::SessionFile;

/// The rows `anima agents threads` shows when `--limit` is not given.
pub const DEFAULT_LIMIT: usize = 30;

/// The most rows one list may hold.
pub const MAX_LIMIT: usize = 500;

/// The shortest ID that `anima agents thread` takes, so a few characters cannot name a thread
/// by chance.
const MIN_ID: usize = 8;

/// A program, as the options name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Program {
    ClaudeCode,
    Codex,
}

impl Program {
    /// The source of the session files of this program.
    #[must_use]
    pub const fn source(self) -> Source {
        match self {
            Self::ClaudeCode => Source::ClaudeCode,
            Self::Codex => Source::Codex,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct ThreadsArgs {
    /// Only the threads of this program.
    #[arg(long, value_enum)]
    program: Option<Program>,
    /// The most threads to show, the last changed first (1 to 500).
    #[arg(long, default_value_t = DEFAULT_LIMIT)]
    limit: usize,
}

#[derive(Debug, clap::Args)]
pub struct ThreadArgs {
    /// The ID of the thread, as `anima agents threads` shows it (8 characters at least).
    id: String,
}

/// One row of the list (`--json` field names).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRow {
    pub id: String,
    pub program: &'static str,
    pub title: Option<String>,
    /// The program or the person gave the title; it is not the start of a prompt.
    pub title_given: bool,
    pub folder: Option<String>,
    pub branch: Option<String>,
    pub model: Option<String>,
    pub started: Option<DateTime<Utc>>,
    pub last: Option<DateTime<Utc>>,
    /// How the file ends: `turn_ended`, `stopped`, `open_tool`, `open` or `nothing` (THR-055).
    pub tail: &'static str,
}

impl ThreadRow {
    /// The row of `thread`, read from `file`; its last time is the later of the last line and
    /// the last change of the file.
    #[must_use]
    pub fn of(file: &SessionFile, thread: Thread) -> Self {
        let modified = file.modified.map(DateTime::<Utc>::from);
        // The ID of a row is the ID that opens it: the ID a Codex file names inside is taken
        // only when the name of the file ends with it; else the name of the file is the ID.
        let id = thread
            .id
            .filter(|id| id.chars().count() >= MIN_ID && file.name.ends_with(id.as_str()))
            .unwrap_or_else(|| file.name.clone());
        Self {
            id,
            program: program_name(file.source),
            title: thread.title,
            title_given: thread.title_given,
            folder: thread.folder,
            branch: thread.branch,
            model: thread.model,
            started: thread.started,
            last: thread.last.max(modified),
            tail: thread.tail.as_str(),
        }
    }
}

/// The last part of a folder path that is not empty, with the separators of each system.
#[must_use]
pub fn folder_name(folder: &str) -> Option<&str> {
    folder.rsplit(['/', '\\']).find(|part| !part.is_empty())
}

/// The words of a program for a person and in JSON.
#[must_use]
pub const fn program_name(source: Source) -> &'static str {
    match source {
        Source::ClaudeCode => "claude_code",
        Source::Codex => "codex",
    }
}

/// The threads of `home`, the last changed first, at most `limit`.
#[must_use]
pub fn list(home: &Path, program: Option<Source>, limit: usize) -> Vec<ThreadRow> {
    files::find(home)
        .into_iter()
        .filter(|file| program.is_none_or(|program| file.source == program))
        .take(limit.clamp(1, MAX_LIMIT))
        .filter_map(|file| {
            let thread = files::glance(&file).ok()?;
            Some(ThreadRow::of(&file, thread))
        })
        .collect()
}

/// The file of the thread `id` under `home`: the file whose name is the ID or ends with it.
///
/// # Errors
/// [`Failure::Usage`] for an ID shorter than 8 characters, [`Failure::NotFound`] when no file
/// or more than one file has it.
pub fn find(home: &Path, id: &str) -> Result<SessionFile, Failure> {
    if id.chars().count() < MIN_ID || id.contains(['/', '\\']) {
        return Err(Failure::Usage {
            message: format!("A thread ID has {MIN_ID} characters at least and no slash."),
            hint: Some(Hint::run("anima agents threads")),
        });
    }
    let mut found = files::find(home)
        .into_iter()
        .filter(|file| file.name == id || file.name.ends_with(id));
    match (found.next(), found.next()) {
        (Some(file), None) => Ok(file),
        (None, _) => Err(Failure::NotFound {
            message: "No thread of Claude Code or Codex on this computer has this ID.".to_owned(),
            hint: Some(Hint::run("anima agents threads")),
        }),
        (Some(_), Some(_)) => Err(Failure::NotFound {
            message: "More than one thread has this ID: give more of it.".to_owned(),
            hint: Some(Hint::run("anima agents threads")),
        }),
    }
}

/// The home folder whose session files are read.
///
/// # Errors
/// [`Failure::NotFound`] when `HOME` is not set to an absolute path.
pub fn home() -> Result<PathBuf, Failure> {
    process_home().ok_or_else(|| Failure::NotFound {
        message: "The home folder of this user is not known (HOME is not set).".to_owned(),
        hint: None,
    })
}

impl ThreadsArgs {
    /// Lists the threads of this computer.
    ///
    /// # Errors
    /// [`Failure`] when the home folder is not known or the output fails.
    pub fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let rows = list(&home()?, self.program.map(Program::source), self.limit);
        invocation.output.data(&rows, |rows| table(rows))
    }
}

impl ThreadArgs {
    /// Shows one thread of this computer.
    ///
    /// # Errors
    /// [`Failure`] for an ID that names no thread, a file that cannot be read, or the output.
    pub fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let file = find(&home()?, &self.id)?;
        let thread = files::read(&file).map_err(|error| Failure::Input {
            what: "the session file".to_owned(),
            reason: error.kind().to_string(),
        })?;
        if invocation.output.json() {
            invocation
                .output
                .data(&shown::Shown::of(&file, &thread), |_| Table::default())
        } else {
            invocation
                .output
                .file(&shown::text(&ThreadRow::of(&file, thread.clone()), &thread))
        }
    }
}

fn table(rows: &[ThreadRow]) -> Table {
    let mut table = Table::new(vec!["LAST", "PROGRAM", "TITLE", "FOLDER", "BRANCH", "ID"]);
    for row in rows {
        let last = row.last.map_or_else(
            || "-".to_owned(),
            |at| at.format("%Y-%m-%d %H:%M").to_string(),
        );
        let folder = row.folder.as_deref().and_then(folder_name).unwrap_or("-");
        table.row(vec![
            Cell::new(&last),
            Cell::toned(shown::program_words(row.program), Tone::Muted),
            Cell::new(row.title.as_deref().unwrap_or("-")),
            Cell::new(folder),
            Cell::new(row.branch.as_deref().unwrap_or("-")),
            Cell::toned(&row.id, Tone::Muted),
        ]);
    }
    table
}
