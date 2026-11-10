//! Whether the recording root can take a recording: set, a folder, writable, with 1 GiB free
//! (K4 §1 step 2, §11; D-14). Free space is read with `statvfs` (G-3).

use std::{io, path::Path};

use rustix::fs::Access;

use crate::sessions::serving::Recording;

/// The least free space a recording starts or writes a part with (K4 §1 step 2, §11).
pub(super) const MIN_FREE: u64 = 1 << 30;

/// Reads the bytes free to the hub under a folder. [`free_space`] in the hub; tests pass
/// their own reading.
pub(super) type FreeSpace = fn(&Path) -> io::Result<u64>;

/// Why the hub cannot record now. The texts hold no path (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(in crate::sessions) enum CannotRecord {
    /// `ANIMA_RECORDING_ROOT` is empty.
    #[error("no recording folder is set")]
    Unset,
    #[error("the recording folder could not be read: {0}")]
    Unreadable(io::ErrorKind),
    #[error("the recording folder is not a folder")]
    NotFolder,
    #[error("the recording folder is not writable")]
    NotWritable,
    #[error("the recording folder has less than 1 GiB free")]
    LowSpace,
}

/// Whether a session can be recorded now (K4 §1 step 2). Three system calls on the root; it
/// reads no file.
///
/// # Errors
/// [`CannotRecord`] names the first condition that fails.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "startSession (WP-48 T5a) is the first caller")
)]
pub(in crate::sessions) fn can_record(recording: &Recording) -> Result<(), CannotRecord> {
    check(recording, free_space)
}

/// [`can_record`] with the free space read by `free`.
pub(super) fn check(recording: &Recording, free: FreeSpace) -> Result<(), CannotRecord> {
    let root = recording.root.as_deref().ok_or(CannotRecord::Unset)?;
    let metadata =
        std::fs::metadata(root).map_err(|error| CannotRecord::Unreadable(error.kind()))?;
    if !metadata.is_dir() {
        return Err(CannotRecord::NotFolder);
    }
    rustix::fs::access(root, Access::WRITE_OK | Access::EXEC_OK)
        .map_err(|_| CannotRecord::NotWritable)?;
    let free = free(root).map_err(|error| CannotRecord::Unreadable(error.kind()))?;
    if free < MIN_FREE {
        return Err(CannotRecord::LowSpace);
    }
    Ok(())
}

/// The bytes an unprivileged writer may still use on the file system holding `folder`.
///
/// # Errors
/// The `statvfs` error.
pub(super) fn free_space(folder: &Path) -> io::Result<u64> {
    let stat = rustix::fs::statvfs(folder)?;
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}
