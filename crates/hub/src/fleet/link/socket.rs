//! The hub's client of the link's control socket: one connection for each request, checked
//! before it opens ([`owned::check`]) and after (the peer runs as the hub's user), the request
//! line out and the reply line in, all within [`CALL_LIMIT`].

use std::{io::ErrorKind, path::PathBuf, time::Duration};

use serde::de::DeserializeOwned;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::UnixStream,
};
use zeroize::Zeroizing;

use super::{
    Error, owned,
    wire::{MAX_LINE, Request, parse_reply},
};

/// How long one call may take, from the check to the reply line. The link answers from memory;
/// a link that takes longer is stuck.
pub(super) const CALL_LIMIT: Duration = Duration::from_secs(5);

/// The link's control socket at one path.
#[derive(Clone, Debug)]
pub(super) struct Control {
    path: PathBuf,
}

impl Control {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Sends `request` and reads its reply.
    ///
    /// # Errors
    /// [`Error`] for an exposed or absent socket, a failed or slow exchange, an unreadable
    /// reply, or the link's refusal.
    pub async fn call<T: DeserializeOwned>(&self, request: Request<'_>) -> Result<T, Error> {
        let line = tokio::time::timeout(CALL_LIMIT, self.exchange(request))
            .await
            .map_err(|_| Error::TimedOut)??;
        parse_reply(&line)
    }

    async fn exchange(&self, request: Request<'_>) -> Result<Zeroizing<Vec<u8>>, Error> {
        let uid = rustix::process::geteuid().as_raw();
        owned::check(&self.path, uid)?;
        let mut stream =
            UnixStream::connect(&self.path)
                .await
                .map_err(|error| match error.kind() {
                    ErrorKind::NotFound | ErrorKind::ConnectionRefused => Error::NotRunning,
                    _ => Error::Connection,
                })?;
        let peer = stream.peer_cred().map_err(|_| Error::Connection)?;
        if peer.uid() != uid {
            return Err(Error::Exposed("the socket's process runs as another user"));
        }
        stream
            .write_all(&request.line())
            .await
            .map_err(|_| Error::Connection)?;
        read_line(&mut stream).await
    }
}

/// The first line of `stream`, at most [`MAX_LINE`] bytes; EOF before any byte, or a longer
/// line, is unreadable. The bytes go into one buffer of the full size, which never grows and
/// is zeroed when it drops, so no copy of an invite is left (LOG-007).
async fn read_line(stream: &mut UnixStream) -> Result<Zeroizing<Vec<u8>>, Error> {
    let mut line = Zeroizing::new(vec![0_u8; MAX_LINE + 1]);
    let mut filled = 0;
    let end = loop {
        let read = stream
            .read(&mut line[filled..])
            .await
            .map_err(|_| Error::Connection)?;
        let newline = line[filled..filled + read].iter().position(|&b| b == b'\n');
        filled += read;
        match newline {
            Some(at) => break filled - read + at,
            None if read == 0 || filled == line.len() => break filled,
            None => {}
        }
    };
    line.truncate(end);
    if line.is_empty() || line.len() > MAX_LINE {
        return Err(Error::Unreadable);
    }
    Ok(line)
}
