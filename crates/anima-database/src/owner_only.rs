//! Folders and files that only the user who runs the database can read (DB-002, DB-003).

use std::fs::{self, DirBuilder, File, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

#[cfg(test)]
mod tests;

/// Makes `path` and its missing parents with mode 0700, and sets 0700 on `path` when it was
/// there already with a wider mode.
///
/// # Errors
/// When the folder cannot be made, or `path` is a symbolic link, not a folder, or a folder of
/// another user: each could put the database's files where somebody else can read or swap them.
pub fn folder(path: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(path)?;
    let found = fs::symlink_metadata(path)?;
    if !found.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a database folder is a symbolic link or not a folder",
        ));
    }
    if found.uid() != nix::unistd::Uid::effective().as_raw() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "a database folder belongs to another user",
        ));
    }
    fs::set_permissions(path, Permissions::from_mode(0o700))
}

/// A new file with mode 0600, open for writing. A file that was at `path` is removed first
/// and never written to: a crash can leave one, and it may hold a secret.
///
/// # Errors
/// When the old file cannot be removed or the new one cannot be made.
pub fn new_file(path: &Path) -> io::Result<File> {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != io::ErrorKind::NotFound
    {
        return Err(error);
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

/// The file at `path` open for appending, made with mode 0600 when it is missing and set to
/// 0600 when it was there.
///
/// # Errors
/// When the file cannot be opened or its mode cannot be set, or `path` is a symbolic link.
pub fn appended_file(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(nix::fcntl::OFlag::O_NOFOLLOW.bits())
        .open(path)?;
    file.set_permissions(Permissions::from_mode(0o600))?;
    Ok(file)
}
