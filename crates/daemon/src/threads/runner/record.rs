//! The record of the runner (THR-030, THR-047): the threads it started, their open requests and
//! the soft limits, in a file of mode 0600, so that a runner that starts again finds them and
//! resumes only its own threads.

use std::io::Write as _;
use std::path::Path;

use agent_threads::control::Limits;
use serde::{Deserialize, Serialize};

use super::RunnerError;
use super::thread::Saved;

/// The lock file of the runner in the folder of the threads: one runner for one folder.
const LOCK: &str = "runner.lock";

/// The name of the record in the folder of the threads.
pub const FILE: &str = "threads.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub limits: Limits,
    pub threads: Vec<Saved>,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            limits: Limits::FIRST,
            threads: Vec::new(),
        }
    }
}

/// The record in `dir`; an empty one when there is none. A record that does not read is a
/// fault: the runner does not start over it, so that no thread is lost from view.
///
/// # Errors
/// The I/O error, or [`std::io::ErrorKind::InvalidData`] for a record that does not read.
pub fn read(dir: &Path) -> std::io::Result<Record> {
    match std::fs::read(dir.join(FILE)) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Record::default()),
        Err(error) => Err(error),
    }
}

/// Writes the record: to a new file of mode 0600 first, then in place of the old one.
///
/// # Errors
/// The I/O error of the file.
pub fn write(dir: &Path, record: &Record) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let text = serde_json::to_vec(record).map_err(std::io::Error::other)?;
    let new = dir.join(format!("{FILE}.new"));
    // A file left by an earlier write may have a different mode: a new one is made 0600.
    match std::fs::remove_file(&new) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&new)?;
    file.write_all(&text)?;
    file.sync_all()?;
    std::fs::rename(new, dir.join(FILE))
}

/// Makes the folder of the threads, and checks that it is a private folder of this user.
pub(super) fn private_folder(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    if !dir.exists() {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(dir)?;
    }
    if crate::config::private_file::folder_private(dir)? {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "the folder of the threads is not a private folder of this user",
        ))
    }
}

/// Takes the lock of the folder: a second runner on the same folder ends at once, so that two
/// runners never stop the programs of each other (THR-113).
pub(super) fn lock(dir: &Path) -> Result<std::fs::File, RunnerError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(dir.join(LOCK))?;
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| RunnerError::Busy)?;
    Ok(file)
}
