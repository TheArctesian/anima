//! The folders of model work (Section 10): made with mode 0700, and refused when a different
//! user owns one or a link replaces it.

use std::fs::DirBuilder;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _};
use std::path::{Component, Path};

use rustix::fs::{Mode, OFlags};

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

/// Makes `relative` below the folder `root` (one of the three), each part relative to an open
/// handle of its parent with `O_NOFOLLOW`, with mode 0700, and owned by this user (Section 10).
/// A link at any part is refused. Gives the handle of the last folder.
///
/// # Errors
/// An I/O error; `PermissionDenied` for a part that a different user owns; `InvalidInput` for
/// a part that is not a plain name.
pub fn private_below(root: &Path, relative: &Path) -> io::Result<OwnedFd> {
    private_folder(root)?;
    let flags = OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC;
    let mut dir = rustix::fs::open(root, flags, Mode::empty())?;
    for part in relative.components() {
        let Component::Normal(name) = part else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a part of a folder of model work is not a name",
            ));
        };
        let made = match rustix::fs::mkdirat(&dir, name, Mode::from_raw_mode(0o700)) {
            Ok(()) => true,
            Err(rustix::io::Errno::EXIST) => false,
            Err(error) => return Err(error.into()),
        };
        let next = rustix::fs::openat(&dir, name, flags, Mode::empty())?;
        if made {
            rustix::fs::fchmod(&next, Mode::from_raw_mode(0o700))?;
        }
        if rustix::fs::fstat(&next)?.st_uid != rustix::process::geteuid().as_raw() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "a folder of model work is not a folder of this user",
            ));
        }
        dir = next;
    }
    Ok(dir)
}

/// The bytes that this user may still write on the file system of `path`.
///
/// # Errors
/// An I/O error of `statvfs`.
pub fn free_bytes(path: &Path) -> io::Result<u64> {
    let stat = rustix::fs::statvfs(path)?;
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
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
