//! The proxy's side of the one interface to the hub ([`crate::wire`]). I/O edge.

use std::{future::Future, io, path::PathBuf};

use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt as _, BufReader},
    net::UnixStream,
};
use uuid::Uuid;

use crate::wire::{self, Answer, Counts, Request, Table, WireError};

/// What the proxy may ask of the hub. A connection names an application, never a port.
pub trait Hub: Send + Sync + 'static {
    type Stream: AsyncRead + AsyncWrite + Unpin + Send;

    /// The names that serve now, with their certificates.
    fn table(&self) -> impl Future<Output = Result<Table, WireError>> + Send;
    /// A connection to the port of `application`, as the hub's record gives it.
    fn connect(&self, application: Uuid) -> impl Future<Output = io::Result<Self::Stream>> + Send;
    /// What the proxy counted for `name`.
    fn count(
        &self,
        name: &str,
        counts: Counts,
    ) -> impl Future<Output = Result<(), WireError>> + Send;
}

/// The hub at its Unix socket.
pub struct HubSocket(pub PathBuf);

impl HubSocket {
    async fn ask(&self, request: &Request) -> Result<BufReader<UnixStream>, WireError> {
        let mut stream = BufReader::new(UnixStream::connect(&self.0).await?);
        wire::write_json(&mut stream, request).await?;
        Ok(stream)
    }
}

impl Hub for HubSocket {
    type Stream = BufReader<UnixStream>;

    async fn table(&self) -> Result<Table, WireError> {
        let mut stream = self.ask(&Request::Table).await?;
        let line = wire::read_line(&mut stream, wire::TABLE_MAX)
            .await?
            .ok_or(WireError::Malformed)?;
        serde_json::from_str(&line).map_err(|_| WireError::Malformed)
    }

    async fn connect(&self, application: Uuid) -> io::Result<Self::Stream> {
        let mut stream = self
            .ask(&Request::Connect { application })
            .await
            .map_err(io::Error::other)?;
        let word = wire::read_line(&mut stream, 32)
            .await
            .map_err(io::Error::other)?
            .unwrap_or_default();
        match Answer::from_word(&word) {
            Some(Answer::Ok) => Ok(stream),
            Some(Answer::Refused) => Err(io::ErrorKind::PermissionDenied.into()),
            _ => Err(io::ErrorKind::NotConnected.into()),
        }
    }

    async fn count(&self, name: &str, counts: Counts) -> Result<(), WireError> {
        let request = Request::Count {
            name: name.to_owned(),
            counts,
        };
        let mut stream = self.ask(&request).await?;
        stream.shutdown().await.ok();
        Ok(())
    }
}
