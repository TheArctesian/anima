//! Windows file security: reject reparse points, inspect ACLs by handle, and create
//! protected directories before a caller writes sensitive bytes (WIN2, CMD-012).
use crate::native;
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

fn open(path: &Path, access: u32) -> io::Result<File> {
    let file = OpenOptions::new()
        .access_mode(access)
        .custom_flags(0x0020_0000 | 0x0200_0000)
        .open(path)?;
    if file.metadata()?.file_attributes() & 0x400 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(file)
}

/// Whether a file or directory has no access by other users (private), or no writes
/// by other users. SYSTEM and administrators are trusted as root is on Unix.
/// # Errors
/// The path cannot be opened or its security descriptor cannot be read.
pub fn secure(path: &Path, private: bool) -> io::Result<bool> {
    native::check(&open(path, 0x0002_0080)?, private)
}

/// Open a regular file without following a reparse point and check its actual handle.
/// # Errors
/// Missing, exposed, non-regular, unreadable or reparse-point files are refused.
pub fn read(path: &Path, private: bool) -> io::Result<File> {
    let file = open(path, 0x8002_0000)?;
    if !file.metadata()?.is_file() || !native::check(&file, private)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(file)
}

/// Create a new regular file inheriting an already private, trusted directory's ACL.
/// # Errors
/// The directory is exposed or replaceable, or the file already exists/cannot be created.
pub fn create_new(path: &Path) -> io::Result<File> {
    let parent = std::path::absolute(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    directory(&parent)?;
    if !secure(&parent, true)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Protect a file or directory with an inheritable, non-inherited user/SYSTEM/admin ACL.
/// # Errors
/// The path is a reparse point, cannot be opened, or the ACL cannot be applied.
pub fn protect(path: &Path) -> io::Result<()> {
    native::protect(&open(path, 0x0006_0080)?)
}

/// Make a private directory tree. Existing directories must not be replaceable by another user.
/// # Errors
/// A path is exposed, a reparse point, or creation/protection fails.
pub fn directory(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    match fs::symlink_metadata(path) {
        Ok(stat) => {
            if !stat.is_dir() || !secure(path, false)? {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            for ancestor in path.ancestors().skip(1) {
                if !secure(ancestor, false)? {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
            }
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            directory(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
            fs::create_dir(path)?;
            protect(path)
        }
        Err(error) => Err(error),
    }
}

/// Preserve a configuration file's ACL on its replacement before writing it.
/// # Errors
/// The source is writable by another user, or either path or ACL is refused.
pub fn copy_acl(source: &Path, destination: &Path) -> io::Result<()> {
    native::copy_acl(
        &open(source, 0x0002_0080)?,
        &open(destination, 0x0006_0080)?,
    )
}

/// Atomically replace a private file after its contents have been flushed by the caller.
/// # Errors
/// Either parent is untrusted, the source is exposed, or Windows cannot publish the file.
pub fn rename_flushed(from: &Path, to: &Path) -> io::Result<()> {
    for path in [from, to] {
        let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
        directory(parent)?;
        if !secure(parent, true)? {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    read(from, true)?;
    native::rename::flushed(from, to)
}

/// Open a checked regular file that has exactly one filesystem name.
/// # Errors
/// Untrusted files and hard links are refused before any contents are read.
pub fn read_single(path: &Path) -> io::Result<File> {
    let file = read(path, false)?;
    if !native::single_name(&file)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(file)
}
