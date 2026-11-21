//! Private Windows `AF_UNIX` entrance. The endpoint and every ancestor are ACL-checked;
//! only this user, SYSTEM and administrators can claim a link device address.
#[path = "header.rs"]
mod header;
use super::Error;
use anima_windows::socket::{Listener, Stream};
use std::{io, net::SocketAddr, path::Path};
use tokio::io::AsyncReadExt;

pub struct LinkSocket(Listener);

/// Bind the link entrance within an existing private directory.
/// # Errors
/// The path is untrusted, a regular file occupies it, or binding fails.
pub fn link_socket(path: &Path) -> Result<LinkSocket, Error> {
    Listener::bind(path)
        .map(LinkSocket)
        .map_err(|error| Error::LinkSocket {
            path: path.to_owned(),
            kind: error.kind(),
        })
}

/// Bind the entrance of the hub's own daemon (HMD-071, HM-T35) by the same rules.
/// # Errors
/// As [`link_socket`].
pub fn own_daemon_socket(path: &Path) -> Result<LinkSocket, Error> {
    Listener::bind(path)
        .map(LinkSocket)
        .map_err(|error| Error::OwnDaemonSocket {
            path: path.to_owned(),
            kind: error.kind(),
        })
}

impl LinkSocket {
    pub(super) async fn accept(&self) -> io::Result<FromLink> {
        self.0.accept().await.map(FromLink)
    }
}

pub(super) struct FromLink(Stream);

impl FromLink {
    /// The connection of the hub's own daemon: the ACL lets only the hub's user reach it.
    pub(super) fn own(self) -> Option<Stream> {
        Some(self.0)
    }

    pub(super) async fn named(self) -> Option<(Stream, SocketAddr)> {
        let Self(mut stream) = self;
        let mut bytes = [0; header::LENGTH];
        stream.read_exact(&mut bytes).await.ok()?;
        Some((stream, header::device(&bytes)?))
    }
}
