//! One change of the hub at a time: `install`, `update`, `remove`, `start` and `stop` of one
//! hub root hold this lock. It is not the lock of `run`, which the hub holds while it runs.

use std::fs::{File, TryLockError};

use anima_database::owner_only;

use super::install::{InstallError, write_error};
use crate::Folders;

/// Held until it is dropped; the operating system also ends it with the process.
#[derive(Debug)]
pub struct Change(#[expect(dead_code, reason = "the open file is the lock")] File);

/// Takes the lock, and makes the hub's folder first when it is missing. It does not wait.
///
/// # Errors
/// [`InstallError::Busy`] when another change holds it.
pub fn take(folders: &Folders) -> Result<Change, InstallError> {
    owner_only::folder(folders.root()).map_err(write_error(folders.root()))?;
    let path = folders.change_lock();
    let file = owner_only::appended_file(&path).map_err(write_error(&path))?;
    match file.try_lock() {
        Ok(()) => Ok(Change(file)),
        Err(TryLockError::WouldBlock) => Err(InstallError::Busy),
        Err(TryLockError::Error(error)) => Err(write_error(&path)(error)),
    }
}
