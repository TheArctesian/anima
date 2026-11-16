//! The hub's client of the link's control socket: one connection for each request, checked
//! before it opens ([`owned::check`]) and after ([`peer_is`]), the request line out and the
//! reply line in, all within [`CALL_LIMIT`].

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

/// The first size of the reply buffer: a reply that is not a long device list fits.
const FIRST_READ: usize = 8 * 1024;

/// The link's control socket at one path, which the user `uid` (the hub's own) must own and
/// serve.
#[derive(Clone, Debug)]
pub(super) struct Control {
    path: PathBuf,
    uid: u32,
}

impl Control {
    pub fn new(path: PathBuf, uid: u32) -> Self {
        Self { path, uid }
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
        let (path, uid) = (self.path.clone(), self.uid);
        // The check reads file metadata and may run `/bin/ls`: off the async workers.
        tokio::task::spawn_blocking(move || owned::check(&path, uid))
            .await
            .map_err(|_| Error::Connection)??;
        let mut stream =
            UnixStream::connect(&self.path)
                .await
                .map_err(|error| match error.kind() {
                    ErrorKind::NotFound | ErrorKind::ConnectionRefused => Error::NotRunning,
                    _ => Error::Connection,
                })?;
        peer_is(&stream, self.uid)?;
        stream
            .write_all(&request.line())
            .await
            .map_err(|_| Error::Connection)?;
        read_line(&mut stream).await
    }
}

/// The process at the other end of `stream` runs as the user `uid`.
///
/// # Errors
/// `Exposed` for another user's process; `Connection` when the system does not say.
pub(super) fn peer_is(stream: &UnixStream, uid: u32) -> Result<(), Error> {
    let peer = stream.peer_cred().map_err(|_| Error::Connection)?;
    if peer.uid() == uid {
        Ok(())
    } else {
        Err(Error::Exposed("the socket's process runs as another user"))
    }
}

/// The first line of `stream`, at most [`MAX_LINE`] bytes; EOF before any byte, or a longer
/// line, is unreadable. The buffer grows by doubling up to the cap; each larger buffer is a
/// fresh copy and the old one is zeroed when it drops, so no copy of an invite is left
/// (LOG-007).
pub(super) async fn read_line(stream: &mut UnixStream) -> Result<Zeroizing<Vec<u8>>, Error> {
    let mut line = Zeroizing::new(vec![0_u8; FIRST_READ]);
    let mut filled = 0;
    let end = loop {
        if filled == line.len() {
            if line.len() > MAX_LINE {
                break filled;
            }
            line = grown(&line, (line.len() * 2).min(MAX_LINE + 1));
        }
        let read = stream
            .read(&mut line[filled..])
            .await
            .map_err(|_| Error::Connection)?;
        let newline = line[filled..filled + read].iter().position(|&b| b == b'\n');
        filled += read;
        match newline {
            Some(at) => break filled - read + at,
            None if read == 0 => break filled,
            None => {}
        }
    };
    line.truncate(end);
    if line.is_empty() || line.len() > MAX_LINE {
        return Err(Error::Unreadable);
    }
    Ok(line)
}

/// A zeroed buffer of `size` bytes that starts with `old`.
fn grown(old: &[u8], size: usize) -> Zeroizing<Vec<u8>> {
    let mut next = Zeroizing::new(vec![0_u8; size]);
    next[..old.len()].copy_from_slice(old);
    next
}
