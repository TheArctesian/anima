//! The folders of model work (Section 10): made with mode 0700, and refused when a different
//! user owns one or a link replaces it.

use std::fs::DirBuilder;
use std::io;
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _};
use std::path::Path;

use daemon_core::model_work::folders::Folders;

/// Makes `path` (and its missing parents) with mode 0700, then checks it.
///
/// # Errors
/// An I/O error, or `PermissionDenied` when the folder is a link, not a folder, or owned by a
/// different user.
pub fn private_folder(path: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(path)?;
    let meta = std::fs::symlink_metadata(path)?;
    let own = meta.uid() == rustix::process::geteuid().as_raw();
    if meta.file_type().is_symlink() || !meta.is_dir() || !own {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "a folder of model work is not a folder of this user",
        ));
    }
    Ok(())
}

/// Makes the three folders.
///
/// # Errors
/// As [`private_folder`].
pub fn make_all(folders: &Folders) -> io::Result<()> {
    [&folders.runtimes, &folders.models, &folders.state]
        .into_iter()
        .try_for_each(|path| private_folder(path))
}

/// The bytes of the files below `path`, with no link followed.
#[must_use]
pub fn bytes_below(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => bytes_below(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map_or(0, |meta| meta.len()),
            _ => 0,
        })
        .sum()
}
