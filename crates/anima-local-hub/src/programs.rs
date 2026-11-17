//! The copy of the programs that the unit runs (DBL-013). The application bundle is replaced
//! by an update while the hub runs; this copy is replaced only while nothing runs.
//!
//! ```text
//! <root>/programs/next/      `copy` writes here; never run
//! <root>/programs/current/   `put_in_place` moves `next` here
//! <root>/programs/previous/  the copy before, until `put_in_place` removes it
//! ```

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read as _};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use anima_database::{Tree, owner_only};

use crate::Folders;
use crate::members::{Carried, LOCAL_HUB, PROGRAMS};

pub mod digest;
#[cfg(test)]
pub(crate) mod tests;
pub mod version;

/// More entries or deeper folders than a PostgreSQL tree has by far (about 1 000 and 6).
const MOST_ENTRIES: usize = 20_000;
const MOST_DEPTH: usize = 16;
/// More bytes than the four programs and a tree have by far (about 60 MB optimised).
const MOST_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum CopyError {
    #[error("the application does not carry {0}")]
    NotCarried(PathBuf),
    #[error("{0} is not a file, a folder or a link inside the PostgreSQL tree")]
    Refused(PathBuf),
    #[error("the PostgreSQL tree is larger than a tree can be")]
    TooLarge,
    #[error("the copied folder is not a PostgreSQL tree ({0})")]
    NotATree(String),
    #[error("cannot {step} {} ({kind})", path.display())]
    Io {
        step: &'static str,
        path: PathBuf,
        kind: io::ErrorKind,
    },
}

fn io_error<'a>(step: &'static str, path: &'a Path) -> impl FnOnce(io::Error) -> CopyError + 'a {
    move |error| CopyError::Io {
        step,
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}

/// Copies the five programs and the PostgreSQL tree of `source` to `programs/next`, in place
/// of a copy that an earlier try left there. Nothing that runs is touched.
///
/// # Errors
/// When a program is missing, the tree holds something that is not a file, a folder or a
/// link that stays inside it, or a file cannot be read or written.
pub fn copy(source: &Carried, folders: &Folders) -> Result<(), CopyError> {
    let next = folders.programs_being_copied();
    owner_only::folder(&folders.all_programs())
        .map_err(io_error("make", &folders.all_programs()))?;
    remove_folder(&next)?;
    let mut budget = Budget {
        entries: MOST_ENTRIES,
        bytes: MOST_BYTES,
    };
    let copied = Carried::installed_in(&next);
    owner_only::folder(copied.programs()).map_err(io_error("make", copied.programs()))?;
    for name in PROGRAMS {
        let from = source.program(name);
        if !from.symlink_metadata().is_ok_and(|found| found.is_file()) {
            return Err(CopyError::NotCarried(from));
        }
        copy_file(&from, &copied.program(name), 0o700, &mut budget)?;
    }
    copy_folder(source.tree(), copied.tree(), MOST_DEPTH, &mut budget)?;
    Tree::at(copied.tree()).map_err(|error| CopyError::NotATree(error.to_string()))?;
    // The digest of what was copied, not of the source: `update` compares the source to it.
    digest::write(&next, &digest::of(&copied)?)
}

/// Makes the copy in `programs/next` the one the unit runs. The caller knows that nothing
/// runs from `programs/current`.
///
/// # Errors
/// When a folder cannot be moved or removed.
pub fn put_in_place(folders: &Folders) -> Result<(), CopyError> {
    let (next, current, previous) = (
        folders.programs_being_copied(),
        folders.programs(),
        folders.programs_replaced(),
    );
    remove_folder(&previous)?;
    let had_current = current.symlink_metadata().is_ok();
    if had_current {
        fs::rename(&current, &previous).map_err(io_error("move", &current))?;
    }
    if let Err(error) = fs::rename(&next, &current) {
        // The unit must not be left with no programs: the ones before go back.
        if had_current {
            fs::rename(&previous, &current).map_err(io_error("move", &previous))?;
        }
        return Err(io_error("move", &next)(error));
    }
    remove_folder(&previous)
}

/// Puts `programs/previous` back as `programs/current` when a stop of the computer or a kill
/// came between the two moves of [`put_in_place`] and left no `current`. The caller holds the
/// lock of the change.
///
/// # Errors
/// When the folder cannot be moved.
pub fn recover(folders: &Folders) -> Result<(), CopyError> {
    let (current, previous) = (folders.programs(), folders.programs_replaced());
    if current.symlink_metadata().is_err() && previous.symlink_metadata().is_ok() {
        fs::rename(&previous, &current).map_err(io_error("move", &previous))?;
    }
    Ok(())
}

/// Removes every copy of the programs. The database, the key and the logs are elsewhere.
///
/// # Errors
/// When the folder cannot be removed.
pub fn remove(folders: &Folders) -> Result<(), CopyError> {
    remove_folder(&folders.all_programs())
}

/// Whether the copy the unit runs is there.
#[must_use]
pub fn are_in_place(folders: &Folders) -> bool {
    Carried::installed_in(&folders.programs())
        .program(LOCAL_HUB)
        .is_file()
}

fn remove_folder(folder: &Path) -> Result<(), CopyError> {
    match fs::remove_dir_all(folder) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("remove", folder)(error)),
    }
}

/// What a copy may still take.
struct Budget {
    entries: usize,
    bytes: u64,
}

fn copy_folder(from: &Path, to: &Path, depth: usize, budget: &mut Budget) -> Result<(), CopyError> {
    if depth == 0 {
        return Err(CopyError::TooLarge);
    }
    DirBuilder::new()
        .mode(0o700)
        .create(to)
        .map_err(io_error("make", to))?;
    for entry in fs::read_dir(from).map_err(io_error("read", from))? {
        let entry = entry.map_err(io_error("read", from))?;
        budget.entries = budget.entries.checked_sub(1).ok_or(CopyError::TooLarge)?;
        let (from, to) = (entry.path(), to.join(entry.file_name()));
        let kind = entry.file_type().map_err(io_error("read", &from))?;
        if kind.is_dir() {
            copy_folder(&from, &to, depth - 1, budget)?;
        } else if kind.is_file() {
            let found = entry.metadata().map_err(io_error("read", &from))?;
            let runs = found.permissions().mode() & 0o111 != 0;
            copy_file(&from, &to, if runs { 0o700 } else { 0o600 }, budget)?;
        } else if kind.is_symlink() {
            copy_link(&from, &to)?;
        } else {
            return Err(CopyError::Refused(from));
        }
    }
    Ok(())
}

/// A library's short name is a link to the file beside it. A link that names a path outside
/// its own folder could make the copy run a file that is not in the copy.
fn copy_link(from: &Path, to: &Path) -> Result<(), CopyError> {
    let target = link_target(from)?;
    std::os::unix::fs::symlink(&target, to).map_err(io_error("make", to))
}

/// The target of the link at `from`, when it names a file beside the link.
fn link_target(from: &Path) -> Result<PathBuf, CopyError> {
    let target = fs::read_link(from).map_err(io_error("read", from))?;
    let stays_beside = matches!(
        target.components().collect::<Vec<_>>().as_slice(),
        [Component::Normal(_)]
    );
    if !stays_beside {
        return Err(CopyError::Refused(from.to_path_buf()));
    }
    Ok(target)
}

fn copy_file(from: &Path, to: &Path, mode: u32, budget: &mut Budget) -> Result<(), CopyError> {
    use nix::fcntl::OFlag;
    // No link is followed, and a pipe that took the file's place does not hold the copy.
    let source = OpenOptions::new()
        .read(true)
        .custom_flags((OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK).bits())
        .open(from)
        .map_err(io_error("read", from))?;
    let found = source.metadata().map_err(io_error("read", from))?;
    if !found.is_file() {
        return Err(CopyError::Refused(from.to_path_buf()));
    }
    budget.bytes = budget
        .bytes
        .checked_sub(found.len())
        .ok_or(CopyError::TooLarge)?;
    let mut copy: File = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(to)
        .map_err(io_error("write", to))?;
    // No more than the size that was counted, also when the file grows meanwhile.
    io::copy(&mut source.take(found.len()), &mut copy).map_err(io_error("write", to))?;
    // The mode of a new file is cut by the umask; the unit must be able to run a program.
    copy.set_permissions(fs::Permissions::from_mode(mode))
        .map_err(io_error("write", to))?;
    copy.sync_all().map_err(io_error("write", to))
}
