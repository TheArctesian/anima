//! Deadlines on what a client sends and reads after its request head (SEC-001, NET-002): a
//! request body must arrive whole within its deadline, and a connection whose writes make no
//! progress for a while is closed, so neither a trickled body nor a client that stops reading
//! holds a place for ever.

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_util::stream;
use http_body_util::BodyExt as _;
use hyper::body::Incoming;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    time::{Sleep, sleep},
};

use super::slots::Slot;

/// Why a request body read through [`timed_body`] ended early.
#[derive(Debug, thiserror::Error)]
pub(super) enum BodyFault {
    #[error("the request body did not arrive in time")]
    TimedOut,
    #[error("the request body could not be read")]
    Read(#[from] hyper::Error),
}

/// `body` as a body that fails once `within` has passed from now, however steadily it
/// trickles in. Trailers are dropped; no route reads them.
pub(super) fn timed_body(body: Incoming, within: Duration) -> axum::body::Body {
    let deadline = Box::pin(sleep(within));
    let chunks = stream::unfold((body, deadline), |(mut body, mut deadline)| async move {
        let next = tokio::select! {
            biased;
            () = &mut deadline => Err(BodyFault::TimedOut),
            frame = body.frame() => match frame? {
                Ok(frame) => Ok(frame.into_data().unwrap_or_default()),
                Err(error) => Err(BodyFault::Read(error)),
            },
        };
        Some((next, (body, deadline)))
    });
    axum::body::Body::from_stream(chunks)
}

/// A connection and its place: an upgraded connection keeps the place until it closes. A
/// write that makes no progress for `stall` fails, which closes the connection.
pub(super) struct Holding<T> {
    io: T,
    _slot: Slot,
    stall: Duration,
    stalled: Option<Pin<Box<Sleep>>>,
}

impl<T> Holding<T> {
    pub(super) fn new(io: T, slot: Slot, stall: Duration) -> Self {
        Self {
            io,
            _slot: slot,
            stall,
            stalled: None,
        }
    }

    /// `written` as it is, but a write still pending `stall` after the last progress fails.
    fn watch<R>(
        &mut self,
        cx: &mut Context<'_>,
        written: Poll<io::Result<R>>,
    ) -> Poll<io::Result<R>> {
        if written.is_ready() {
            self.stalled = None;
            return written;
        }
        let stall = self.stall;
        let since = self.stalled.get_or_insert_with(|| Box::pin(sleep(stall)));
        if since.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(io::ErrorKind::TimedOut.into()));
        }
        Poll::Pending
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for Holding<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_read(cx, buf)
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Holding<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let written = Pin::new(&mut this.io).poll_write(cx, buf);
        this.watch(cx, written)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let flushed = Pin::new(&mut this.io).poll_flush(cx);
        this.watch(cx, flushed)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let shut = Pin::new(&mut this.io).poll_shutdown(cx);
        this.watch(cx, shut)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let written = Pin::new(&mut this.io).poll_write_vectored(cx, bufs);
        this.watch(cx, written)
    }

    fn is_write_vectored(&self) -> bool {
        self.io.is_write_vectored()
    }
}
