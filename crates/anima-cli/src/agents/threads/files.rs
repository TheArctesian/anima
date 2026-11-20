//! The session files of Claude Code and Codex of this user, on this computer (THR-003,
//! THR-007): only the names below, never a link, and never a settings or credential file of
//! the programs. A folder on the way from the home folder to a session file must be a real
//! folder, and a session file must belong to this user and have one name (no hard link). This
//! is the I/O edge; `daemon_core::agent_threads` reads the lines.
//!
//! - `.claude/projects/<project>/<id>.jsonl`: the session `<id>` (sub-agents are not threads);
//! - `.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`: a Codex thread, named inside the file.

use std::fs::{self, DirEntry, File};
use std::io::{self, BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use daemon_core::agent_threads::{Thread, ThreadReader};
use daemon_core::agent_usage::Source;

/// The most files one walk lists, so a folder full of files cannot hold the command.
const MAX_FILES: usize = 100_000;

/// The deepest a rollout is under `.codex/sessions` (`YYYY/MM/DD/rollout-*.jsonl`).
const CODEX_DEPTH: usize = 4;

/// The longest line read; a longer line (a pasted image) is skipped.
const MAX_LINE: u64 = 16 * 1024 * 1024;

/// The longest line a glance reads: a title or a folder is in a short line.
const GLANCE_LINE: u64 = 64 * 1024;

/// How much of the start and of the end of a file a glance reads: the start names the folder
/// and the first prompt, the end the title the person gave and the last time.
const GLANCE_HEAD: u64 = 256 * 1024;
const GLANCE_TAIL: u64 = 64 * 1024;

/// One session file.
#[derive(Clone, Debug)]
pub struct SessionFile {
    pub path: PathBuf,
    pub source: Source,
    /// The ID from the file name: the session of Claude Code, the rollout name of Codex.
    pub name: String,
    pub modified: Option<SystemTime>,
}

/// Every session file under `home`, the last changed first, at most [`MAX_FILES`]. A folder
/// that is missing or cannot be read gives none.
#[must_use]
pub fn find(home: &Path) -> Vec<SessionFile> {
    let mut found = Vec::new();
    let claude = real_folder(home, &[".claude", "projects"]);
    let codex = real_folder(home, &[".codex", "sessions"]);
    for project in claude.iter().flat_map(|folder| entries(folder)) {
        if is_folder(&project) {
            for entry in entries(&project.path()) {
                if found.len() < MAX_FILES && named(&entry, "") {
                    found.extend(file(&entry, Source::ClaudeCode));
                }
            }
        }
    }
    if let Some(codex) = codex {
        walk(&codex, CODEX_DEPTH, &mut |entry| {
            if found.len() < MAX_FILES && named(entry, "rollout-") {
                found.extend(file(entry, Source::Codex));
            }
        });
    }
    found.sort_by_key(|file| std::cmp::Reverse(file.modified));
    found
}

/// The whole thread of `file`.
///
/// # Errors
/// The I/O error of opening or reading the file.
pub fn read(file: &SessionFile) -> io::Result<Thread> {
    let mut reader = ThreadReader::new(file.source);
    let mut source = BufReader::with_capacity(64 * 1024, open_no_link(&file.path)?);
    feed(&mut source, u64::MAX, MAX_LINE, &mut reader)?;
    Ok(named_thread(file, reader.finish()))
}

/// The start and the end of `file`: enough for a row of the list (the title, the folder, the
/// branch, the model, the times), not for the entries.
///
/// # Errors
/// As [`read`].
pub fn glance(file: &SessionFile) -> io::Result<Thread> {
    let mut reader = ThreadReader::new(file.source);
    let mut opened = open_no_link(&file.path)?;
    let size = opened.metadata()?.len();
    feed(
        &mut BufReader::new(&opened),
        GLANCE_HEAD,
        GLANCE_LINE,
        &mut reader,
    )?;
    if size > GLANCE_HEAD + GLANCE_TAIL {
        opened.seek(SeekFrom::Start(size - GLANCE_TAIL))?;
        let mut tail = BufReader::new(&opened);
        // The first line of the tail starts in the middle: skip it.
        tail.read_until(b'\n', &mut Vec::new())?;
        feed(&mut tail, GLANCE_TAIL, GLANCE_LINE, &mut reader)?;
    }
    Ok(named_thread(file, reader.finish()))
}

/// A thread with the ID of its file when the lines named none.
fn named_thread(file: &SessionFile, mut thread: Thread) -> Thread {
    thread.id.get_or_insert_with(|| file.name.clone());
    thread
}

/// Feeds `reader` each complete line of `source` of at most `longest` bytes, until `limit`
/// bytes are read; a longer line is passed over.
fn feed(
    source: &mut impl io::BufRead,
    limit: u64,
    longest: u64,
    reader: &mut ThreadReader,
) -> io::Result<()> {
    let mut read: u64 = 0;
    let mut line = Vec::new();
    while read < limit {
        line.clear();
        let length = source.take(longest + 1).read_until(b'\n', &mut line)? as u64;
        if length == 0 {
            break;
        }
        read += length;
        match line.last() {
            Some(b'\n') => reader.feed(&line[..line.len() - 1]),
            _ if length > longest => {
                read += skip_line(source)?;
            }
            // A line the program is still writing.
            _ => break,
        }
    }
    Ok(())
}

/// Skips past the next newline; the bytes passed.
fn skip_line(source: &mut impl io::BufRead) -> io::Result<u64> {
    let mut passed: u64 = 0;
    loop {
        let buffer = source.fill_buf()?;
        if buffer.is_empty() {
            return Ok(passed);
        }
        if let Some(at) = buffer.iter().position(|byte| *byte == b'\n') {
            source.consume(at + 1);
            return Ok(passed + at as u64 + 1);
        }
        let length = buffer.len();
        source.consume(length);
        passed += length as u64;
    }
}

/// Opens the regular file at `path` for reading without following a link in its last part
/// (`O_NOFOLLOW`), and without waiting on a pipe (`O_NONBLOCK`).
fn open_no_link(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = File::from(rustix::fs::open(path, flags, Mode::empty())?);
    let metadata = file.metadata()?;
    let own = metadata.uid() == rustix::process::getuid().as_raw();
    if !metadata.is_file() || !own || metadata.nlink() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file of this user with one name",
        ));
    }
    Ok(file)
}

fn file(entry: &DirEntry, source: Source) -> Option<SessionFile> {
    // `DirEntry::metadata` does not follow a link.
    let metadata = entry.metadata().ok()?;
    let path = entry.path();
    let name = path.file_stem()?.to_str()?.to_owned();
    Some(SessionFile {
        path,
        source,
        name,
        modified: metadata.modified().ok(),
    })
}

/// `home` with `parts` below it, when each part is a real folder and not a link.
fn real_folder(home: &Path, parts: &[&str]) -> Option<PathBuf> {
    let mut folder = home.to_path_buf();
    for part in parts {
        folder.push(part);
        if !fs::symlink_metadata(&folder).is_ok_and(|kind| kind.is_dir()) {
            return None;
        }
    }
    Some(folder)
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
