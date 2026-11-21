//! One request from the client to the application and its answer back (Section 12.3). The head
//! is the one that [`crate::head::rewrite`] wrote; the body goes as its `Content-Length` says and
//! no byte more, so a second request on the same connection never reaches the application
//! unread. The body must come before the deadline of its request ([`Body::by`]), so a client that
//! sends one byte at a time cannot hold the stream. A WebSocket upgrade goes both ways as it is
//! after the application's `101`. Each stream can end after a time with no byte ([`Idle`]).
//! Nothing here reads or logs a body.

use std::{
    future::Future as _,
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use tokio::{
    io::{
        AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, ReadBuf, copy,
        copy_bidirectional,
    },
    time::{Instant, Sleep, sleep, timeout_at},
};

use crate::head::Rewritten;

/// The most bytes read for the status line of an answer.
const STATUS_MAX: usize = 16 * 1024;

/// What one request moved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Relayed {
    /// Bytes from the client to the application, the head included.
    pub bytes_in: u64,
    /// Bytes from the application to the client.
    pub bytes_out: u64,
    /// The status of the application's answer; `None` for an answer that does not start with a
    /// status line.
    pub status: Option<u16>,
}

/// The body of a request as the relay gets it.
#[derive(Clone, Copy, Debug)]
pub struct Body<'a> {
    /// The bytes of the body that came with the head.
    pub early: &'a [u8],
    /// The time by which the whole body must have come (SF-NEW-revroutelimits).
    pub by: Instant,
}

/// Sends the request to `app` and its answer to `client`. A WebSocket upgrade goes both ways
/// only when the application answers `101`; any other answer goes back as a plain one and the
/// connection ends (SF-NEW-revrouteupgrade).
///
/// # Errors
/// An I/O error of either side; [`io::ErrorKind::TimedOut`] when the body is not all there by
/// [`Body::by`].
pub async fn relay<C, A>(
    client: &mut C,
    body: Body<'_>,
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
    let early = body.early;
    if !request.upgrade {
        let first = early
            .len()
            .min(usize::try_from(request.body).unwrap_or(usize::MAX));
        app.write_all(&early[..first]).await?;
        let rest = request.body - first as u64;
        let sent = timeout_at(body.by, copy(&mut (&mut *client).take(rest), app))
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
        if sent < rest {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        relayed.bytes_in += request.body;
    }
    app.flush().await?;
    let first = status_line(app).await?;
    relayed.status = status(&first);
    client.write_all(&first).await?;
    relayed.bytes_out = first.len() as u64;
    if request.upgrade && relayed.status == Some(101) {
        app.write_all(early).await?;
        let (up, down) = copy_bidirectional(client, app).await?;
        relayed.bytes_in += early.len() as u64 + up;
        relayed.bytes_out += down;
        return Ok(relayed);
    }
    relayed.bytes_out += copy(app, client).await?;
    client.flush().await?;
    Ok(relayed)
}

/// The first bytes of the answer of the application, up to the end of its status line or
/// [`STATUS_MAX`] bytes, whichever comes first.
async fn status_line<A: AsyncRead + Unpin>(app: &mut A) -> io::Result<Vec<u8>> {
    let mut first = Vec::with_capacity(STATUS_MAX);
    let mut chunk = [0_u8; 1024];
    while first.len() < STATUS_MAX && !first.windows(2).any(|pair| pair == b"\r\n") {
        let read = app.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        first.extend_from_slice(&chunk[..read]);
    }
    Ok(first)
}

/// The status code of `HTTP/1.x NNN ...` at the start of `bytes`.
#[must_use]
pub fn status(bytes: &[u8]) -> Option<u16> {
    let line = bytes.strip_prefix(b"HTTP/1.")?;
    let code = line.get(2..5)?;
    std::str::from_utf8(code).ok()?.parse().ok()
}

/// A stream that fails with [`io::ErrorKind::TimedOut`] when no byte moved through it, in either
/// direction, for its idle time (SF-NEW-revroutelimits).
pub struct Idle<S> {
    inner: S,
    after: Duration,
    deadline: Pin<Box<Sleep>>,
}

impl<S> Idle<S> {
    #[must_use]
    pub fn new(inner: S, after: Duration) -> Self {
        Self {
            inner,
            after,
            deadline: Box::pin(sleep(after)),
        }
    }

    fn moved(&mut self) {
        let next = Instant::now() + self.after;
        self.deadline.as_mut().reset(next);
    }

    /// `Pending` stays pending until the idle time is over, then it is an error.
    fn waiting<T>(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<T>> {
        match self.deadline.as_mut().poll(cx) {
            Poll::Ready(()) => Poll::Ready(Err(io::ErrorKind::TimedOut.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Idle<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = &mut *self;
        let before = buf.filled().len();
        match Pin::new(&mut this.inner).poll_read(cx, buf) {
            Poll::Ready(result) => {
                if buf.filled().len() > before {
                    this.moved();
                }
                Poll::Ready(result)
            }
            Poll::Pending => this.waiting(cx),
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Idle<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = &mut *self;
        match Pin::new(&mut this.inner).poll_write(cx, bytes) {
            Poll::Ready(Ok(written)) => {
                if written > 0 {
                    this.moved();
                }
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => this.waiting(cx),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = &mut *self;
        match Pin::new(&mut this.inner).poll_flush(cx) {
            Poll::Pending => this.waiting(cx),
            Poll::Ready(result) => Poll::Ready(result),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
