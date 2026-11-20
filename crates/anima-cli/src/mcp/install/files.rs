//! The configuration file of another program, read and replaced with care (MCP-031, MCP-032,
//! MCP-034): no symbolic link, no file that a different user can change, a bounded size, a copy
//! of the file as it was, and a rename so that a reader never sees half of it.

use std::fs;
#[cfg(unix)]
use std::fs::{File, OpenOptions};
use std::io::{self, Read as _, Write as _};
#[cfg(unix)]
use std::os::unix::fs::{
    DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
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
    #[error("a different program changed it at the same time; try again")]
    Changed,
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
    if below
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(FileError::NotAFile);
    }
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
        if shared(&at, &metadata, root.user)? {
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
    #[cfg(unix)]
    let opened = OpenOptions::new()
        .read(true)
        .custom_flags(no_follow())
        .open(path);
    #[cfg(windows)]
    let opened = anima_windows::files::read(path, false);
    let file = match opened {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    // The file that was opened, not the name: the name can change after the check.
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(FileError::NotAFile);
    }
    #[cfg(unix)]
    if open_to_others(&metadata, root.user) {
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
#[cfg(unix)]
fn no_follow() -> i32 {
    i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits()).unwrap_or_default()
}

/// Whether a different user can change what `metadata` describes: a different owner, a write bit
/// for all, or a write bit for a group that is not the private group of the user (the group
/// with the user's own number, which the usual Linux systems give each user).
#[cfg(unix)]
#[must_use]
pub fn open_to_others(metadata: &fs::Metadata, user: u32) -> bool {
    let mode = metadata.permissions().mode();
    metadata.uid() != user || mode & 0o002 != 0 || (mode & 0o020 != 0 && metadata.gid() != user)
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
    // A folder that is made is private: the mask of the process does not decide (MCP-034).
    #[cfg(unix)]
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(folder)?;
    #[cfg(windows)]
    anima_windows::files::directory(folder)?;
    check_path(root, folder.join(name).as_path())?;
    let mode = permissions(path);
    if let Some(before) = before {
        keep_copy(root, path, mode, before)?;
    }
    let mut temporary = temporary(folder, path, mode)?;
    temporary.write_all(text.as_bytes())?;
    temporary.as_file().sync_all()?;
    // The file is as it was read, or nothing is written: a program that saved it in between keeps its text.
    if read(root, path)?.as_deref() != before {
        return Err(FileError::Changed);
    }
    // A rename replaces the name and follows no link at it.
    temporary
        .persist(path)
        .map_err(|error| FileError::Io(error.error))?;
    #[cfg(unix)]
    File::open(folder).and_then(|folder| folder.sync_all()).ok();
    Ok(())
}

/// Keeps `before` beside `path` as the copy of MCP-032, unless a copy is there. A copy that is
/// there must be a regular file of the user: a link or a file of a different user is refused, so
/// that "a copy is kept" is never said of something else. The copy appears whole or not at all.
fn keep_copy(root: Root<'_>, path: &Path, mode: u32, before: &str) -> Result<(), FileError> {
    let (folder, _) = parts(path)?;
    let copy = copy_path(path);
    match fs::symlink_metadata(&copy) {
        Ok(metadata) if metadata.is_file() && !shared(&copy, &metadata, root.user)? => {
            return Ok(());
        }
        Ok(metadata) if metadata.file_type().is_symlink() => return Err(FileError::Link),
        Ok(_) => return Err(FileError::Shared),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut temporary = temporary(folder, path, mode)?;
    temporary.write_all(before.as_bytes())?;
    temporary.as_file().sync_all()?;
    // A hard link fails when the name is there: a copy made in the meantime is kept.
    match fs::hard_link(temporary.path(), &copy) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(all(test, unix))]
mod tests;

#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "Windows ACL inspection can fail; both platforms share this fallible boundary"
)]
fn shared(_: &Path, metadata: &fs::Metadata, user: u32) -> io::Result<bool> {
    Ok(open_to_others(metadata, user))
}
#[cfg(windows)]
fn shared(path: &Path, _: &fs::Metadata, _: u32) -> io::Result<bool> {
    anima_windows::files::secure(path, false).map(|secure| !secure)
}
#[cfg(unix)]
fn permissions(path: &Path) -> u32 {
    fs::symlink_metadata(path).map_or(0o600, |metadata| metadata.permissions().mode() & 0o777)
}
#[cfg(windows)]
fn permissions(_: &Path) -> u32 {
    0
}

fn temporary(folder: &Path, original: &Path, mode: u32) -> io::Result<tempfile::NamedTempFile> {
    #[cfg(unix)]
    let file = {
        let _ = original;
        tempfile::Builder::new()
            .prefix(".anima-")
            .permissions(fs::Permissions::from_mode(mode))
            .tempfile_in(folder)?
    };
    #[cfg(windows)]
    let file = {
        let _ = mode;
        let file = tempfile::Builder::new()
            .prefix(".anima-")
            .tempfile_in(folder)?;
        match fs::symlink_metadata(original) {
            Ok(_) => anima_windows::files::copy_acl(original, file.path())?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                anima_windows::files::protect(file.path())?;
            }
            Err(error) => return Err(error),
        }
        file
    };
    Ok(file)
}
