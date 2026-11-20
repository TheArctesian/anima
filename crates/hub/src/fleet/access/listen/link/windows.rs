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

impl LinkSocket {
    pub(super) async fn accept(&self) -> io::Result<FromLink> {
        self.0.accept().await.map(FromLink)
    }
}

pub(super) struct FromLink(Stream);

impl FromLink {
    pub(super) async fn named(self) -> Option<(Stream, SocketAddr)> {
        let Self(mut stream) = self;
        let mut bytes = [0; header::LENGTH];
        stream.read_exact(&mut bytes).await.ok()?;
        Some((stream, header::device(&bytes)?))
    }
}
