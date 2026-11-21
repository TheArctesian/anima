//! The record of the runner (THR-030, THR-047): the threads it started, their open requests and
//! the soft limits, in a file of mode 0600, so that a runner that starts again finds them and
//! resumes only its own threads.

use std::io::Write as _;
use std::path::Path;

use agent_threads::control::Limits;
use serde::{Deserialize, Serialize};

use super::thread::Saved;

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
    let new = dir.join(format!("{FILE}.new"));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&new)?;
    file.write_all(&serde_json::to_vec(record).unwrap_or_default())?;
    file.sync_all()?;
    std::fs::rename(new, dir.join(FILE))
}
