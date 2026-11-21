//! The record of the holder: for each running program, its thread, its process ID and the start
//! time of the process, in a file of mode 0600 of the user (H9). A holder that starts ends each
//! process an older holder left, if the start time is the same, so that it never ends a process
//! that took an old ID.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::tree;
use super::wire::ThreadKey;
use crate::config::private_file;

const FILE: &str = "holder.json";

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub version: u32,
    pub threads: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub thread: ThreadKey,
    pub pid: u32,
    pub started: Option<String>,
}

/// Writes the record in place of the old one.
///
/// # Errors
/// The error of the file when the record cannot be written.
pub fn write(dir: &Path, record: &Record) -> Result<(), private_file::PrivateFileError> {
    let bytes = serde_json::to_vec(record).unwrap_or_default();
    private_file::write(dir, FILE, &bytes)
}

/// The record that is in `dir`, if one is there and reads.
#[must_use]
pub fn read(dir: &Path) -> Option<Record> {
    let bytes = private_file::read(&dir.join(FILE)).ok()??;
    serde_json::from_slice(&bytes).ok()
}

/// Ends each program that an older holder recorded and that still runs with the same start time,
/// with the processes below it, and empties the record.
pub async fn end_left_over(dir: &Path) {
    let Some(old) = read(dir) else { return };
    for entry in old.threads {
        let same = entry.started.is_some() && tree::started(entry.pid).await == entry.started;
        if same {
            tree::end(entry.pid, true, &tree::Known::new(), Duration::from_secs(3)).await;
        }
    }
    let _ = write(
        dir,
        &Record {
            version: old.version,
            threads: Vec::new(),
        },
    );
}
