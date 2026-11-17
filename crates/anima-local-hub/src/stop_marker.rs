//! The mark that the operator asked for a stop. An update replaces the programs and leaves a
//! hub stopped that the operator stopped; `start`, `install`, `remove` and a starting `run`
//! take the mark away.

use std::fs;
use std::io;

use anima_database::owner_only;

use crate::Folders;

/// Writes the mark. The hub's folder is made when it is missing.
///
/// # Errors
/// When the folder or the file cannot be made.
pub(crate) fn set(folders: &Folders) -> io::Result<()> {
    owner_only::folder(folders.root())?;
    owner_only::new_file(&folders.stopped_by_operator()).map(drop)
}

/// Takes the mark away; a missing mark is fine.
///
/// # Errors
/// When the file cannot be removed.
pub(crate) fn clear(folders: &Folders) -> io::Result<()> {
    match fs::remove_file(folders.stopped_by_operator()) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Whether the operator's stop is the last word.
pub(crate) fn is_set(folders: &Folders) -> bool {
    folders.stopped_by_operator().symlink_metadata().is_ok()
}
