//! The check before the hub connects to the link's control socket: the rule of `DialOwned` in
//! `link/statedir/statedir.go`, so that nothing the hub sends (an invite request, a revocation)
//! can reach another user. The socket must be the hub user's and owner-only; its folder must be
//! too; no folder above may be another user's or open to others without the sticky bit; and a
//! symbolic link in the path passes only when no other user can change where it leads.
//!
//! The check reads file metadata only. Between it and the connect, only this user or root can
//! change what the path leads to.

use std::{
    fs::{self, Metadata},
    io::{self, ErrorKind},
    os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _},
    path::Path,
};

use super::Error;

const SOCKET: &str = "the socket is another user's, or others can reach it";
const FOLDER: &str = "the socket's folder is another user's, or others can reach it";
const ABOVE: &str = "a folder above the socket is another user's, or others can write to it";
const LINK: &str = "a symbolic link in the socket's path can be changed by another user";
const UNREADABLE: &str = "a folder of the socket's path cannot be read";

/// Whether the socket at `path` is safe to connect to for the user `uid`.
///
/// # Errors
/// `NotRunning` when the socket or a folder of its path is not there; `Exposed` with the rule
/// it breaks otherwise.
pub(super) fn check(path: &Path, uid: u32) -> Result<(), Error> {
    let folder = path.parent().ok_or(Error::Exposed(FOLDER))?;
    private(folder, uid)?;
    let socket = fs::symlink_metadata(path).map_err(|error| absent(&error))?;
    if !socket.file_type().is_socket() || !owner_only(&socket, uid) {
        return Err(Error::Exposed(SOCKET));
    }
    Ok(())
}

/// The folder is this user's and owner-only, and every folder above it is safe.
fn private(folder: &Path, uid: u32) -> Result<(), Error> {
    let stat = fs::symlink_metadata(folder).map_err(|error| absent(&error))?;
    if !stat.is_dir() || !owner_only(&stat, uid) {
        return Err(Error::Exposed(FOLDER));
    }
    no_foreign_link_above(folder, uid)?;
    // The folders above are checked on the real path, so a symbolic link cannot hide them.
    let resolved = fs::canonicalize(folder).map_err(|error| absent(&error))?;
    for above in resolved.ancestors().skip(1) {
        let stat = fs::symlink_metadata(above).map_err(|error| absent(&error))?;
        if open_to_others(&stat) || !root_or(&stat, uid) {
            return Err(Error::Exposed(ABOVE));
        }
    }
    Ok(())
}

/// Each symbolic link in the path as written is root's or this user's, in a folder that is
/// root's or this user's and that others cannot write to.
fn no_foreign_link_above(folder: &Path, uid: u32) -> Result<(), Error> {
    for above in folder.ancestors().skip(1) {
        let Some(parent) = above.parent() else {
            return Ok(());
        };
        let stat = fs::symlink_metadata(above).map_err(|error| absent(&error))?;
        if !stat.file_type().is_symlink() {
            continue;
        }
        let holder = fs::symlink_metadata(parent).map_err(|error| absent(&error))?;
        if open_to_others(&holder) || !root_or(&stat, uid) || !root_or(&holder, uid) {
            return Err(Error::Exposed(LINK));
        }
    }
    Ok(())
}

/// Group or others may write it, and the sticky bit does not stop them replacing an entry.
fn open_to_others(stat: &Metadata) -> bool {
    const STICKY: u32 = 0o1000;
    let mode = stat.permissions().mode();
    mode & 0o022 != 0 && mode & STICKY == 0
}

fn root_or(stat: &Metadata, uid: u32) -> bool {
    stat.uid() == 0 || stat.uid() == uid
}

fn owner_only(stat: &Metadata, uid: u32) -> bool {
    const GROUP_AND_OTHERS: u32 = 0o077;
    stat.permissions().mode() & GROUP_AND_OTHERS == 0 && stat.uid() == uid
}

/// A missing part of the path means the link is not running; any other failure to read it
/// means the hub cannot tell, so it sends nothing.
fn absent(error: &io::Error) -> Error {
    if error.kind() == ErrorKind::NotFound {
        Error::NotRunning
    } else {
        Error::Exposed(UNREADABLE)
    }
}
