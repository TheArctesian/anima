//! The Unix socket of `anima-link device`, the daemon's way to the hub in the link mode: the
//! check that only a program of the daemon's own user can listen there (K8 Section 1, R-172).

use std::io;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _};
use std::path::Path;

use crate::config::private_file;

#[derive(Debug, thiserror::Error)]
pub enum LinkSocketError {
    #[error("the folder of the socket of anima-link cannot be read: {0}")]
    Io(#[from] io::Error),
    #[error(
        "the socket of anima-link is not in a folder owned by this user with no access for others, or a thing at its path is not a socket of this user"
    )]
    Exposed,
}

/// Checks that the bearer can go to `socket`: its folder is private to the running user and no
/// other user can replace it, and a thing at the path is a socket that the running user owns.
/// A path with no socket passes: the link is stopped, and only this user can make one there.
///
/// # Errors
/// [`LinkSocketError::Exposed`] when another user could listen at the path;
/// [`LinkSocketError::Io`] when the folder does not exist or cannot be read.
pub fn trusted(socket: &Path) -> Result<(), LinkSocketError> {
    let folder = socket.parent().ok_or(LinkSocketError::Exposed)?;
    if !private_file::folder_private(folder)? {
        return Err(LinkSocketError::Exposed);
    }
    match std::fs::symlink_metadata(socket) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(found)
            if found.file_type().is_socket() && found.uid() == private_file::running_user()? =>
        {
            Ok(())
        }
        Ok(_) => Err(LinkSocketError::Exposed),
    }
}
