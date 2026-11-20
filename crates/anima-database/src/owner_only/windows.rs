//! Private Windows database state. Refuse reparse points and writable ancestors before
//! creating files, and apply the user/SYSTEM/administrator ACL before writing secrets.
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

/// Make a checked private directory tree.
/// # Errors
/// A directory is exposed, is a reparse point, or cannot be made private.
pub fn folder(path: &Path) -> io::Result<()> {
    anima_windows::files::directory(path)?;
    anima_windows::files::protect(path)
}

/// A newly created file inheriting the checked parent's private ACL.
/// # Errors
/// The parent is untrusted or creation fails. An old file is unlinked, never overwritten.
pub fn new_file(path: &Path) -> io::Result<File> {
    private_parent(path)?;
    if let Err(error) = fs::remove_file(path)
        && error.kind() != io::ErrorKind::NotFound
    {
        return Err(error);
    }
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Open a private regular file for append without following Windows reparse points.
/// # Errors
/// The file or its parent is untrusted, or the file cannot be opened/protected.
pub fn appended_file(path: &Path) -> io::Result<File> {
    private_parent(path)?;
    let file = OpenOptions::new()
        .read(true) // LockFileEx requires read or write access beyond append-only access.
        .append(true)
        .create(true)
        .custom_flags(0x0020_0000)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    anima_windows::files::protect(path)?;
    Ok(file)
}

fn private_parent(path: &Path) -> io::Result<()> {
    let parent = std::path::absolute(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    anima_windows::files::directory(&parent)?;
    if !anima_windows::files::secure(&parent, true)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn private_files_replace_and_append_without_losing_their_acl() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state/record");
        new_file(&path).unwrap().write_all(b"secret").unwrap();
        assert!(anima_windows::files::secure(&path, true).unwrap());
        new_file(&path).unwrap().write_all(b"new").unwrap();
        appended_file(&path).unwrap().write_all(b" data").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new data");
        assert!(anima_windows::files::secure(&path, true).unwrap());
    }
}
