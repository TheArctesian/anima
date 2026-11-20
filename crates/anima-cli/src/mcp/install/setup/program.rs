//! Programs used by MCP setup: discovery and replacement-safe executable paths.
use super::Here;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Why `anima` cannot be the program of an entry (MCP-033).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    #[error("the path of anima is not an absolute path of plain text")]
    NotAbsolute,
    #[error("the path of anima is not a program that can be run")]
    NotAProgram,
    #[error("a different user can replace the anima program or its folder")]
    Replaceable,
}

/// The path of `anima` after [`check_command`] accepted it: the only value an entry is made of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked(PathBuf);

impl Checked {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// Whether a different user can replace what is at `path`: the thing itself or a folder above it,
/// up to the root. The system's own folders (owned by root) count as fixed.
#[cfg(unix)]
fn replaceable(path: &Path, user: u32) -> bool {
    path.ancestors().any(|part| {
        fs::symlink_metadata(part).map_or(true, |metadata| {
            let mode = metadata.permissions().mode();
            let owned = metadata.uid() == user || metadata.uid() == 0;
            // A link has no mode of its own; the folder that holds it decides who replaces it. A
            // folder with the sticky bit (as /tmp) lets only the owner of an entry replace it.
            let sticky = metadata.is_dir() && mode & 0o1000 != 0;
            let open = !metadata.file_type().is_symlink()
                && !sticky
                && (mode & 0o002 != 0
                    || (mode & 0o020 != 0 && metadata.gid() != user && metadata.gid() != 0));
            !owned || open
        })
    })
}

/// Checks that `anima` is a fixed program that only this user or the system can replace, and
/// that its path is text that no agent reads as a variable (MCP-033).
///
/// # Errors
/// [`CommandError`] when it is not absolute, not a runnable regular file, or replaceable.
pub fn check_command(anima: &Path, user: u32) -> Result<Checked, CommandError> {
    let text = anima.to_str().ok_or(CommandError::NotAbsolute)?;
    // `$` and braces are variables in the files of some agents; `..` names a different place.
    let plain = !text
        .chars()
        .any(|character| character.is_control() || matches!(character, '$' | '{' | '}'));
    if !anima.is_absolute()
        || !plain
        || anima
            .components()
            .any(|part| part == std::path::Component::ParentDir)
    {
        return Err(CommandError::NotAbsolute);
    }
    // The program may be a link that an installer made (INS-002): the link and what it points at
    // are both checked, each with every folder above it.
    let program = fs::metadata(anima).map_err(|_| CommandError::NotAProgram)?;
    if !runnable(anima, &program) {
        return Err(CommandError::NotAProgram);
    }
    let real = fs::canonicalize(anima).map_err(|_| CommandError::NotAProgram)?;
    if replaceable(anima, user) || replaceable(&real, user) {
        return Err(CommandError::Replaceable);
    }
    Ok(Checked(anima.to_path_buf()))
}

/// The first program named `name` in the folders of `PATH`, as an absolute path.
pub(super) fn program(here: &Here, name: &str) -> Option<PathBuf> {
    here.path
        .iter()
        .filter(|folder| folder.is_absolute())
        .flat_map(|folder| {
            #[cfg(unix)]
            let names = vec![folder.join(name)];
            #[cfg(windows)]
            let names = ["exe", "com", "cmd", "bat"]
                .map(|extension| folder.join(format!("{name}.{extension}")));
            names
        })
        .find(|path| fs::metadata(path).is_ok_and(|metadata| runnable(path, &metadata)))
}

#[cfg(windows)]
fn replaceable(path: &Path, _: u32) -> bool {
    path.ancestors()
        .any(|part| !anima_windows::files::secure(part, false).unwrap_or(false))
}
fn runnable(path: &Path, metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        let _ = path;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        metadata.is_file()
            && path
                .extension()
                .and_then(|part| part.to_str())
                .is_some_and(|part| {
                    ["exe", "com", "cmd", "bat"]
                        .iter()
                        .any(|extension| part.eq_ignore_ascii_case(extension))
                })
    }
}
