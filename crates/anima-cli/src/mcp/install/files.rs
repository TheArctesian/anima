//! The configuration file of another program, read and replaced with care (MCP-031, MCP-032,
//! MCP-034): no symbolic link, no file that a different user can change, a bounded size, a copy
//! of the file as it was, and a rename so that a reader never sees half of it.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

/// The most a configuration file may hold: a hostile file cannot make the reader use more.
pub const LIMIT: u64 = 1024 * 1024;

/// What is put after the name of a file for the copy of it as it was (MCP-032).
pub const COPY_SUFFIX: &str = ".before-anima";

/// Why a file was not read or not written. No variant carries text of the file (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("a part of its path is a symbolic link")]
    Link,
    #[error("it is not a regular file")]
    NotAFile,
    #[error("a different user owns it or can write it or its folder")]
    Shared,
    #[error("it is larger than 1 MiB")]
    TooLarge,
    #[error("it is not UTF-8 text")]
    NotText,
    #[error("{0}")]
    Io(#[from] io::Error),
}

/// The home folder that every configuration file is below, and the user that must own each part.
#[derive(Clone, Copy, Debug)]
pub struct Root<'a> {
    pub home: &'a Path,
    pub user: u32,
}

/// The folder and the name of `path`; a path with neither is refused as not a file.
fn parts(path: &Path) -> Result<(&Path, &std::ffi::OsStr), FileError> {
    match (path.parent(), path.file_name()) {
        (Some(folder), Some(name)) => Ok((folder, name)),
        _ => Err(FileError::NotAFile),
    }
}

/// Refuses a symbolic link at `path` or at a folder between it and `home`, and a folder that is
/// there and that a different user owns or can write. A part that is not there yet is not checked.
fn check_path(root: Root<'_>, path: &Path) -> Result<(), FileError> {
    let below = path
        .strip_prefix(root.home)
        .map_err(|_| FileError::NotAFile)?;
    let mut at = root.home.to_path_buf();
    let count = below.components().count();
    for (position, part) in below.components().enumerate() {
        at.push(part);
        let metadata = match fs::symlink_metadata(&at) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            return Err(FileError::Link);
        }
        let last = position + 1 == count;
        if last != metadata.is_file() {
            return Err(FileError::NotAFile);
        }
        if metadata.uid() != root.user || metadata.permissions().mode() & 0o022 != 0 {
            return Err(FileError::Shared);
        }
    }
    Ok(())
}

/// The text of the file at `path` below `home`, or `None` when it is not there (MCP-034).
///
/// # Errors
/// [`FileError`] for a link, a file that is not regular, shared, too large or not text.
pub fn read(root: Root<'_>, path: &Path) -> Result<Option<String>, FileError> {
    check_path(root, path)?;
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc_nofollow())
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    // The file that was opened, not the name: the name can change after the check.
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(FileError::NotAFile);
    }
    if metadata.uid() != root.user || metadata.permissions().mode() & 0o022 != 0 {
        return Err(FileError::Shared);
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(FileError::TooLarge);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| FileError::NotText)
}

/// `O_NOFOLLOW`: the open fails when the last part of the path is a symbolic link.
const fn libc_nofollow() -> i32 {
    if cfg!(target_os = "macos") {
        0x0100
    } else {
        0o400_000
    }
}

/// The path of the copy of `path` as it was (MCP-032).
#[must_use]
pub fn copy_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(COPY_SUFFIX);
    PathBuf::from(name)
}

/// Replaces the file at `path` with `text`. `before` is the text that [`read`] gave: when it is
/// there and no copy exists yet, it is first kept beside the file (MCP-032). The new file has the
/// permission bits of the old one, or `0600` (MCP-031).
///
/// # Errors
/// [`FileError`] as for [`read`], or when a step of the write fails; the file is then as it was.
pub fn replace(
    root: Root<'_>,
    path: &Path,
    before: Option<&str>,
    text: &str,
) -> Result<(), FileError> {
    check_path(root, path)?;
    let (folder, name) = parts(path)?;
    fs::create_dir_all(folder)?;
    check_path(root, folder.join(name).as_path())?;
    let mode =
        fs::symlink_metadata(path).map_or(0o600, |metadata| metadata.permissions().mode() & 0o777);
    if let Some(before) = before {
        // `create_new` follows no link and fails when a copy is there: the first copy is kept.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(copy_path(path))
        {
            Ok(mut copy) => {
                copy.write_all(before.as_bytes())?;
                copy.sync_all()?;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    let mut temporary = tempfile::Builder::new()
        .prefix(".anima-")
        .permissions(fs::Permissions::from_mode(mode))
        .tempfile_in(folder)?;
    temporary.write_all(text.as_bytes())?;
    temporary.as_file().sync_all()?;
    // A rename replaces the name and follows no link at it.
    temporary
        .persist(path)
        .map_err(|error| FileError::Io(error.error))?;
    File::open(folder).and_then(|folder| folder.sync_all()).ok();
    Ok(())
}

#[cfg(test)]
mod tests;
