//! A copy of the whole root of a stopped hub, with a dump of the database in it, and the check
//! of such a copy (ROL-054, R-189). The copy is what a hosted hub goes back to when an update
//! fails: the database cluster, the vault key, the sealing keys, the sealed files, the state of
//! the link with its key, the logs.
//!
//! ```text
//! <copy>/root/          each file of the hub root, but the programs, the locks and the sockets
//! <copy>/database.dump  `pg_dump` of the hub's database, made directly before the files
//! <copy>/copy.json      the list of the entries with their sizes, modes and SHA-256
//! ```
//!
//! The copy holds every secret of the hub. Its folder and each file in it keep the owner-only
//! modes of the root, and nothing of a file is printed.

mod database;
mod entries;
mod list;

#[cfg(test)]
mod tests;

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anima_database::backup::{self, BackupError};
use anima_database::{Tree, TreeError, owner_only};

use crate::run::{self, RunError};
use crate::{Carried, Folders};
pub use list::{Entry, Kind, List};

/// The dump of the database in a copy.
pub const DUMP: &str = "database.dump";
/// The list of a copy.
pub const LIST: &str = "copy.json";
/// The folder of a copy that holds the files of the root.
pub const ROOT: &str = "root";

#[derive(Debug, thiserror::Error)]
pub enum RootCopyError {
    #[error("the hub of this folder runs: stop it, then make the copy")]
    HubRuns,
    #[error("this folder holds no hub: there is no database below it")]
    NoHub,
    #[error("the place of the copy is there already; give a path that does not exist")]
    PlaceTaken,
    #[error("the folder of the copy can be written by other users")]
    SharedFolder,
    #[error("cannot {step} ({kind})")]
    Io {
        step: &'static str,
        kind: io::ErrorKind,
    },
    #[error("the database did not start for the dump: {0}")]
    Database(String),
    #[error(transparent)]
    Dump(#[from] BackupError),
    #[error(transparent)]
    Tree(#[from] TreeError),
    #[error("the copy has no list that reads; it is not a copy of a hub root, or it is damaged")]
    NoList,
    #[error("the copy is damaged: {0}")]
    Damaged(String),
}

pub(crate) fn io_error(step: &'static str) -> impl FnOnce(io::Error) -> RootCopyError {
    move |error| RootCopyError::Io {
        step,
        kind: error.kind(),
    }
}

/// What a copy holds, for the one line that the command prints. No path and no secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Summary {
    pub entries: usize,
    pub bytes: u64,
    /// When the copy was made, in seconds of Unix time.
    pub made: u64,
}

/// Makes a copy of the root of `folders` in the new folder `to`. The hub must be stopped: this
/// holds the lock of its `run` for the whole time, starts only the database for the dump, and
/// stops it again before it reads a file.
///
/// # Errors
/// [`RootCopyError::HubRuns`] for a hub that runs; otherwise the step that failed. A copy that
/// is not whole is removed.
pub fn make(carried: &Carried, folders: &Folders, to: &Path) -> Result<Summary, RootCopyError> {
    if !folders.database().cluster().is_dir() {
        return Err(RootCopyError::NoHub);
    }
    let _lock = run::lock(folders).map_err(|error| match error {
        RunError::AnotherCopy => RootCopyError::HubRuns,
        RunError::Io { step, kind } => RootCopyError::Io { step, kind },
    })?;
    new_place(to)?;
    let made = fill(carried, folders, to);
    if made.is_err() {
        // A part of a copy is of no use and holds the hub's keys.
        fs::remove_dir_all(to).ok();
    }
    made
}

fn fill(carried: &Carried, folders: &Folders, to: &Path) -> Result<Summary, RootCopyError> {
    database::dump(carried, folders, &to.join(DUMP))?;
    let mut list = entries::copy(folders, &to.join(ROOT))?;
    list.dump = Some(entries::file_entry(&to.join(DUMP), Path::new(DUMP))?);
    list.made = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    list.write(&to.join(LIST))?;
    Ok(list.summary())
}

/// Makes the folder of a new copy, mode 0700, below a folder that only this user can write.
fn new_place(to: &Path) -> Result<(), RootCopyError> {
    if fs::symlink_metadata(to).is_ok() {
        return Err(RootCopyError::PlaceTaken);
    }
    let above = to
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let above = fs::metadata(above).map_err(io_error("look at the folder of the copy"))?;
    if above.permissions().mode() & 0o022 != 0 {
        return Err(RootCopyError::SharedFolder);
    }
    owner_only::folder(to).map_err(io_error("make the folder of the copy"))
}

/// Checks the copy in `copy`: each entry of its list is there with its size, its mode and its
/// SHA-256, the parts that a hub cannot start without are in the list, and the dump reads with
/// the `pg_restore` of the PostgreSQL in `tree`. It starts no server and changes nothing.
///
/// # Errors
/// [`RootCopyError::NoList`] or [`RootCopyError::Damaged`] with the first fault; the fault
/// names a path of the copy and never a content.
pub fn check(tree: &Path, copy: &Path) -> Result<Summary, RootCopyError> {
    let list = List::read(&copy.join(LIST))?;
    list.has_the_parts_of_a_hub()?;
    for entry in &list.entries {
        entries::verify(&copy.join(ROOT), entry)?;
    }
    let dump = list
        .dump
        .as_ref()
        .ok_or_else(|| RootCopyError::Damaged("the list names no dump".to_owned()))?;
    entries::verify(copy, dump)?;
    if !backup::reads(&Tree::at(tree)?, &copy.join(DUMP))? {
        return Err(RootCopyError::Damaged(
            "pg_restore cannot read the dump".to_owned(),
        ));
    }
    Ok(list.summary())
}
