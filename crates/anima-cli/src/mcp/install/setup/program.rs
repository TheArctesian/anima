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

#[cfg(unix)]
/// The group `admin` of macOS. Its members can become root, so a folder of root that this group
/// can write (as `/Applications`) is as fixed as a folder of root alone.
const MACOS_ADMIN: u32 = 80;

/// One part of a path as [`replaceable`] reads it.
#[cfg(unix)]
#[derive(Clone, Copy, Debug)]
pub(super) struct Part {
    pub owner: u32,
    pub group: u32,
    pub mode: u32,
    pub folder: bool,
    pub link: bool,
}

#[cfg(unix)]
/// Whether a different user than `user` can replace `part`. `macos`: the group `admin` counts as
/// root.
pub(super) fn open_to_others(part: Part, user: u32, macos: bool) -> bool {
    let owned = part.owner == user || part.owner == 0;
    // A link has no mode of its own; the folder that holds it decides who replaces it. A folder
    // with the sticky bit (as /tmp) lets only the owner of an entry replace it.
    let sticky = part.folder && part.mode & 0o1000 != 0;
    let fixed_group = part.group == user
        || part.group == 0
        || (macos && part.owner == 0 && part.group == MACOS_ADMIN);
    let open = !part.link
        && !sticky
        && (part.mode & 0o002 != 0 || (part.mode & 0o020 != 0 && !fixed_group));
    !owned || open
}

/// Whether a different user can replace what is at `path`: the thing itself or a folder above it,
/// up to the root. The system's own folders (owned by root) count as fixed.
#[cfg(unix)]
pub(super) fn replaceable(path: &Path, user: u32) -> bool {
    path.ancestors().any(|part| {
        fs::symlink_metadata(part).map_or(true, |metadata| {
            let part = Part {
                owner: metadata.uid(),
                group: metadata.gid(),
                mode: metadata.permissions().mode(),
                folder: metadata.is_dir(),
                link: metadata.file_type().is_symlink(),
            };
            open_to_others(part, user, cfg!(target_os = "macos"))
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

/// An agent's own program that may be run: found on `PATH`, and neither it, what it links to, nor
/// a folder above them can be replaced by a different user (the check of `anima`, MCP-033).
pub(super) enum Found {
    Fixed(PathBuf),
    /// On `PATH`, but a different user can replace it: it is not run.
    Replaceable,
    Missing,
}

/// The first program named `name` on `PATH`, if it is one that may be run (MCP-035).
pub(super) fn fixed_program(here: &Here, name: &str) -> Found {
    let Some(path) = program(here, name) else {
        return Found::Missing;
    };
    let real = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    if replaceable(&path, here.user) || replaceable(&real, here.user) {
        Found::Replaceable
    } else {
        Found::Fixed(path)
    }
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
pub(super) fn replaceable(path: &Path, _: u32) -> bool {
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
