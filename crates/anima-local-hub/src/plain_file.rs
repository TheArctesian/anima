//! Files in the hub's folder that the program opens without trust: a link or a pipe that took
//! the place of one must not block the program or make it read without end. The way is that
//! of the copy: no link followed, no wait for a pipe, and the descriptor must be a regular file.

use std::fs::{File, OpenOptions, Permissions};
use std::io::{self, Read as _};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

use nix::fcntl::OFlag;

fn refuse_what_is_not_a_file(file: &File) -> io::Result<()> {
    if file.metadata()?.is_file() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ))
    }
}

const NO_LINK_NO_WAIT: i32 = OFlag::O_NOFOLLOW.bits() | OFlag::O_NONBLOCK.bits();

/// The file at `path`, made with mode 0600 when it is missing and set to 0600 when it was
/// there, open for appending (the lock of a change keeps it open).
///
/// # Errors
/// When `path` is a link or not a regular file, or the file cannot be opened.
pub(crate) fn appended(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(NO_LINK_NO_WAIT)
        .open(path)?;
    refuse_what_is_not_a_file(&file)?;
    file.set_permissions(Permissions::from_mode(0o600))?;
    Ok(file)
}

/// The text of the file at `path`, which may hold no more than `limit` bytes.
///
/// # Errors
/// When `path` is a link or not a regular file, holds more than `limit` bytes or is not
/// text, or cannot be read.
pub(crate) fn read_text(path: &Path, limit: u64) -> io::Result<String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(NO_LINK_NO_WAIT)
        .open(path)?;
    refuse_what_is_not_a_file(&file)?;
    let mut text = String::new();
    file.take(limit + 1).read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "too long"));
    }
    Ok(text)
}
