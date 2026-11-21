//! One request from the client to the application and its answer back (Section 12.3). The head
//! is the one that [`crate::head::rewrite`] wrote; the body goes as its `Content-Length` says and
//! no byte more, so a second request on the same connection never reaches the application
//! unread. A WebSocket upgrade goes both ways as it is. Nothing here reads or logs a body.

use std::io;

use tokio::io::{
    AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, copy, copy_bidirectional,
};

use crate::head::Rewritten;

/// What one request moved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Relayed {
    /// Bytes from the client to the application, the head included.
    pub bytes_in: u64,
    /// Bytes from the application to the client.
    pub bytes_out: u64,
    /// The status of the application's answer; `None` for an upgrade or an answer that does not
    /// start with a status line.
    pub status: Option<u16>,
}

/// Sends the request to `app` and its answer to `client`. `early` holds the bytes of the body
/// that came with the head.
///
/// # Errors
/// An I/O error of either side.
pub async fn relay<C, A>(
    client: &mut C,
    early: &[u8],
    app: &mut A,
    request: &Rewritten,
) -> io::Result<Relayed>
where
    C: AsyncRead + AsyncWrite + Unpin + Send,
    A: AsyncRead + AsyncWrite + Unpin + Send,
{
    app.write_all(&request.head).await?;
    let mut relayed = Relayed {
        bytes_in: request.head.len() as u64,
        ..Relayed::default()
    };
    if request.upgrade {
        app.write_all(early).await?;
        let (up, down) = copy_bidirectional(client, app).await?;
        relayed.bytes_in += early.len() as u64 + up;
        relayed.bytes_out = down;
        return Ok(relayed);
    }
    let first = early
        .len()
        .min(usize::try_from(request.body).unwrap_or(usize::MAX));
    app.write_all(&early[..first]).await?;
    let rest = request.body - first as u64;
    let sent = copy(&mut (&mut *client).take(rest), app).await?;
    if sent < rest {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    app.flush().await?;
    relayed.bytes_in += request.body;
    let mut buffer = vec![0_u8; 16 * 1024];
    let read = app.read(&mut buffer).await?;
    relayed.status = status(&buffer[..read]);
    client.write_all(&buffer[..read]).await?;
    relayed.bytes_out = read as u64 + copy(app, client).await?;
    client.flush().await?;
    Ok(relayed)
}

/// The status code of `HTTP/1.x NNN ...` at the start of `bytes`.
#[must_use]
pub fn status(bytes: &[u8]) -> Option<u16> {
    let line = bytes.strip_prefix(b"HTTP/1.")?;
    let code = line.get(2..5)?;
    std::str::from_utf8(code).ok()?.parse().ok()
}
