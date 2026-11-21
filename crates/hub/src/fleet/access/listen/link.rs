//! The hub's socket for the link (one-click plan, item S5): a Unix socket that only the hub's
//! user can open, so that only `anima-link` can connect. Each connection starts with a header
//! that names the device by its link address; that address is the connection's peer for
//! every rule that names a client (NET-002, BLD-024).

mod header;
#[cfg(test)]
mod tests;

use std::{
    fs, io,
    net::SocketAddr,
    os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _},
    path::Path,
};

use tokio::{
    io::AsyncReadExt as _,
    net::{UnixListener, UnixStream},
};

use super::Error;

/// A bound private socket, and the user that made it: the hub's. The socket of the link, or
/// the socket of the hub's own daemon in the hosted mode (HMD-071, HM-T35).
pub struct LinkSocket {
    listener: UnixListener,
    owner: u32,
}

/// Makes the socket at `path`, with mode `0600`. Its folder must be a folder of the hub's user
/// that no other user can reach: whoever can open the socket names a device. A socket an
/// earlier hub left at `path` is replaced; any other file there is left alone and refused.
///
/// # Errors
///
/// The folder is missing, shared or another user's; `path` holds a file that is not a socket;
/// or the socket cannot be made.
pub fn link_socket(path: &Path) -> Result<LinkSocket, Error> {
    private_socket(path).map_err(|fault| match fault {
        Fault::Folder => Error::LinkFolder(path.to_owned()),
        Fault::NotSocket => Error::LinkPath(path.to_owned()),
        Fault::Unmade(kind) => Error::LinkSocket {
            path: path.to_owned(),
            kind,
        },
    })
}

/// Makes the socket of the hub's own daemon at `path` (HMD-071, HM-T35), by the rules of
/// [`link_socket`]: whoever can open it pushes as the hub's own device.
///
/// # Errors
///
/// As [`link_socket`].
pub fn own_daemon_socket(path: &Path) -> Result<LinkSocket, Error> {
    private_socket(path).map_err(|fault| match fault {
        Fault::Folder => Error::OwnDaemonFolder(path.to_owned()),
        Fault::NotSocket => Error::OwnDaemonPath(path.to_owned()),
        Fault::Unmade(kind) => Error::OwnDaemonSocket {
            path: path.to_owned(),
            kind,
        },
    })
}

/// Why a private socket was not made.
enum Fault {
    /// The folder is missing, shared or another user's.
    Folder,
    /// A file that is not a socket is at the path.
    NotSocket,
    Unmade(io::ErrorKind),
}

fn private_socket(path: &Path) -> Result<LinkSocket, Fault> {
    let folder = path.parent().ok_or(Fault::Folder)?;
    if !fs::symlink_metadata(folder).is_ok_and(|folder| owner_only(&folder)) {
        return Err(Fault::Folder);
    }
    match fs::symlink_metadata(path) {
        Ok(left) if left.file_type().is_socket() => fs::remove_file(path).map_err(unmade)?,
        Ok(_) => return Err(Fault::NotSocket),
        Err(_) => {}
    }
    let listener = std::os::unix::net::UnixListener::bind(path).map_err(unmade)?;
    let made = fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .and_then(|()| listener.set_nonblocking(true))
        .and_then(|()| Ok((fs::symlink_metadata(path)?, fs::symlink_metadata(folder)?)));
    match made {
        // The new socket belongs to the hub's user, so the folder must have the same owner.
        Ok((socket, folder)) if owner_only(&folder) && folder.uid() == socket.uid() => {
            let listener = UnixListener::from_std(listener).map_err(unmade)?;
            let owner = socket.uid();
            Ok(LinkSocket { listener, owner })
        }
        // A socket that cannot be removed here stays unused; the next start replaces it.
        Ok(_) => {
            let _ = fs::remove_file(path);
            Err(Fault::Folder)
        }
        Err(error) => {
            let _ = fs::remove_file(path);
            Err(unmade(error))
        }
    }
}

/// Whether `folder` is a folder that its owner alone can reach.
#[expect(
    clippy::verbose_bit_mask,
    reason = "the mask is a file mode: no access for the group and for others"
)]
fn owner_only(folder: &fs::Metadata) -> bool {
    folder.is_dir() && folder.mode() & 0o077 == 0
}

#[expect(clippy::needless_pass_by_value, reason = "it is given to map_err")]
fn unmade(error: io::Error) -> Fault {
    Fault::Unmade(error.kind())
}

impl LinkSocket {
    /// The next connection to the socket, and whether a process of the hub's own user made
    /// it. The file modes let no other user connect; this holds also where they do not, for
    /// example under an inherited access-control entry.
    pub(super) async fn accept(&self) -> io::Result<FromLink> {
        let (stream, _) = self.listener.accept().await?;
        let own = stream
            .peer_cred()
            .is_ok_and(|peer| peer.uid() == self.owner);
        Ok(FromLink { stream, own })
    }
}

/// A connection to the link's socket.
pub(super) struct FromLink {
    stream: UnixStream,
    own: bool,
}

impl FromLink {
    /// The connection of the hub's own daemon, which names no device: `None` when another
    /// user's process connected (HMD-071, HM-T35).
    pub(super) fn own(self) -> Option<UnixStream> {
        self.own.then_some(self.stream)
    }

    /// The connection and the device it names as its peer, read from the header at its start.
    /// `None` when another user's process connected, when the connection ends first or when
    /// it does not start with the link's header.
    pub(super) async fn named(self) -> Option<(UnixStream, SocketAddr)> {
        let Self { mut stream, own } = self;
        if !own {
            return None;
        }
        let mut bytes = [0; header::LENGTH];
        stream.read_exact(&mut bytes).await.ok()?;
        Some((stream, header::device(&bytes)?))
    }
}
