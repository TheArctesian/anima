//! What is at `<folder>/anima` now (INS-006): nothing, a link and where it points, a launcher this
//! app wrote and which `AppImage` it runs, or something else. Reads only: it never follows the
//! link and reads at most a few KiB of a file.

use std::{
    fs::{self, OpenOptions},
    io::{self, Read},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use super::{error::Occupant, launcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Nothing,
    Link(PathBuf),
    /// A launcher this app wrote, and the `AppImage` it runs.
    Launcher(PathBuf),
    Other(Occupant),
}

/// What is at `path`. An entry that cannot be read is something, never nothing, so it is not
/// replaced.
pub fn look(path: &Path) -> Found {
    let metadata = match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Found::Nothing,
        Err(_) => return Found::Other(Occupant::Other),
        Ok(metadata) => metadata,
    };
    let kind = metadata.file_type();
    if kind.is_symlink() {
        return fs::read_link(path).map_or(Found::Other(Occupant::Other), Found::Link);
    }
    if kind.is_dir() {
        return Found::Other(Occupant::Folder);
    }
    if !kind.is_file() {
        return Found::Other(Occupant::Other);
    }
    small_text(path)
        .as_deref()
        .and_then(launcher::appimage_of)
        .map_or(Found::Other(Occupant::File), Found::Launcher)
}

/// The text of a small regular file, or `None` when it is larger than any launcher or not UTF-8.
/// Opened without following a link and without blocking, so a pipe put there meanwhile is not
/// waited on.
fn small_text(path: &Path) -> Option<String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut text = String::new();
    file.take(launcher::SIZE_LIMIT + 1)
        .read_to_string(&mut text)
        .ok()?;
    (u64::try_from(text.len()).ok()? <= launcher::SIZE_LIMIT).then_some(text)
}
