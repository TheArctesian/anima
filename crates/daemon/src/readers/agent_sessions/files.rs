//! Finds the session files of Claude Code and Codex under a home folder (AIS-018, K8 Section 4):
//! only the names below, never a link, and never a settings or credential file of the programs
//! (AIS-022).
//!
//! - `.claude/projects/<project>/<id>.jsonl`: the session `<id>`;
//! - `.claude/projects/<project>/<id>/subagents/**/agent-*.jsonl`: its sub-agents (3 levels);
//! - `.codex/sessions/**/rollout-*.jsonl` (4 levels): a Codex thread, named inside the file.

use std::fs::{self, DirEntry, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use daemon_core::agent_usage::Source;

/// The most files one walk lists, so a folder full of files cannot hold the daemon.
const MAX_FILES: usize = 100_000;

/// The deepest a sub-agent file is under `<id>/subagents`.
const SUB_AGENT_DEPTH: usize = 3;

/// The deepest a rollout is under `.codex/sessions` (`YYYY/MM/DD/rollout-*.jsonl`).
const CODEX_DEPTH: usize = 4;

/// One session file and what its size and modification time were.
#[derive(Clone, Debug)]
pub(super) struct Found {
    pub path: PathBuf,
    pub source: Source,
    /// The Claude Code session the file counts in; `None` for Codex, named inside the file.
    pub session: Option<String>,
    /// The session's own file, not a sub-agent's.
    pub main: bool,
    pub stamp: Stamp,
}

/// The size and modification time of a file: when both are as before, it is not read again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// Opens the regular file at `path` for reading without following a link in its last part
/// (`O_NOFOLLOW`), so a link swapped in after the walk is refused, and without waiting on a
/// pipe (`O_NONBLOCK`).
///
/// # Errors
/// The error of the open, or `InvalidInput` when the path is not a regular file.
pub(super) fn open_no_link(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = File::from(rustix::fs::open(path, flags, Mode::empty())?);
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    Ok(file)
}

/// Every session file under `home`, at most [`MAX_FILES`]. A folder that is missing or cannot
/// be read gives none.
pub(super) fn find(home: &Path) -> Vec<Found> {
    let mut found = Vec::new();
    for project in folders(&home.join(".claude/projects")) {
        for entry in entries(&project) {
            claude_entry(&entry, &mut found);
        }
    }
    walk(&home.join(".codex/sessions"), CODEX_DEPTH, &mut |entry| {
        if found.len() < MAX_FILES && named(entry, "rollout-") {
            found.extend(file(entry, Source::Codex, None, true));
        }
    });
    found
}

/// A session file of a project, or the folder of its sub-agents.
fn claude_entry(entry: &DirEntry, found: &mut Vec<Found>) {
    let path = entry.path();
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return;
    };
    let session = Some(stem.to_owned());
    if found.len() >= MAX_FILES {
        return;
    }
    if named(entry, "") {
        found.extend(file(entry, Source::ClaudeCode, session, true));
    } else if is_folder(entry) {
        walk(&path.join("subagents"), SUB_AGENT_DEPTH, &mut |agent| {
            if found.len() < MAX_FILES && named(agent, "agent-") {
                found.extend(file(agent, Source::ClaudeCode, session.clone(), false));
            }
        });
    }
}

/// The file of `entry`, with its size and modification time; `None` when they cannot be read.
fn file(entry: &DirEntry, source: Source, session: Option<String>, main: bool) -> Option<Found> {
    // `DirEntry::metadata` does not follow a link.
    let metadata = entry.metadata().ok()?;
    Some(Found {
        path: entry.path(),
        source,
        session,
        main,
        stamp: Stamp {
            size: metadata.len(),
            modified: metadata.modified().ok(),
        },
    })
}

/// Calls `visit` with each regular file under `folder`, `depth` levels down at most.
fn walk(folder: &Path, depth: usize, visit: &mut dyn FnMut(&DirEntry)) {
    for entry in entries(folder) {
        if is_folder(&entry) {
            if depth > 1 {
                walk(&entry.path(), depth - 1, visit);
            }
        } else {
            visit(&entry);
        }
    }
}

/// The real folders (not links) in `folder`.
fn folders(folder: &Path) -> Vec<PathBuf> {
    entries(folder)
        .iter()
        .filter(|entry| is_folder(entry))
        .map(DirEntry::path)
        .collect()
}

fn entries(folder: &Path) -> Vec<DirEntry> {
    fs::read_dir(folder)
        .map_or_default(|read| read.filter_map(Result::ok).take(MAX_FILES).collect())
}

fn is_folder(entry: &DirEntry) -> bool {
    entry.file_type().is_ok_and(|kind| kind.is_dir())
}

/// A regular file (not a link) named `<prefix>*.jsonl`.
fn named(entry: &DirEntry, prefix: &str) -> bool {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    entry.file_type().is_ok_and(|kind| kind.is_file())
        && name.starts_with(prefix)
        && name.ends_with(".jsonl")
}
