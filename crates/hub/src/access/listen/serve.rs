//! The way in for every connection (NET-002, SEC-001): the peer is checked against the
//! permitted networks at TCP accept, before any TLS work, and a refused peer is closed at once.
//! Caps in total and per peer bound connections and handshakes; deadlines bound the handshake
//! and every request head, including the wait for the next request on a kept-alive connection.
//!
//! The accept loop never waits on a client: each connection runs in its own task, and the
//! tasks are aborted when serving stops.

use std::{
    convert::Infallible,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use axum::{Router, extract::ConnectInfo};
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    task::JoinSet,
};
use tokio_rustls::TlsAcceptor;
use tower::ServiceExt as _;

use super::slots::{Slot, Slots};
use crate::access::{
    layer::{CONNECTION, Edge, Verdict},
    networks,
};

/// The caps and deadlines of the way in.
#[derive(Clone, Copy, Debug)]
pub(super) struct Limits {
    /// Connections open at once, upgraded ones included.
    pub connections: usize,
    pub connections_per_peer: usize,
    pub handshakes_per_peer: usize,
    /// A TLS handshake that takes longer is dropped.
    pub handshake_time: Duration,
    /// A request head that takes longer, counted from the end of the last answer or the
    /// handshake, closes the connection; so does an idle kept-alive connection.
    pub header_time: Duration,
}

/// The limits the hub serves with.
pub(super) const SERVED: Limits = Limits {
    connections: 1024,
    connections_per_peer: 64,
    handshakes_per_peer: 8,
    handshake_time: Duration::from_secs(5),
    header_time: Duration::from_secs(10),
};

/// After an accept error other than a reset connection (for example too many open files).
const ACCEPT_PAUSE: Duration = Duration::from_secs(1);

/// What every connection is served with.
pub(super) struct Serving {
    router: Router,
    edge: Edge,
    tls: Option<TlsAcceptor>,
    limits: Limits,
    connections: Arc<Slots>,
    handshakes: Arc<Slots>,
}

impl Serving {
    pub(super) fn new(
        router: Router,
        edge: Edge,
        tls: Option<TlsAcceptor>,
        limits: Limits,
    ) -> Arc<Self> {
        Arc::new(Self {
            router,
            edge,
            tls,
            limits,
            connections: Slots::new(limits.connections, limits.connections_per_peer),
            handshakes: Slots::new(limits.connections, limits.handshakes_per_peer),
        })
    }

    /// The work of serving `stream` from `peer`, or `None` when the caps leave no place for
    /// it; then `stream` is dropped, which closes it.
    pub(super) fn admit(
        self: &Arc<Self>,
        stream: TcpStream,
        peer: SocketAddr,
    ) -> Option<impl Future<Output = ()> + Send + 'static> {
        let peer = SocketAddr::new(networks::canonical(peer.ip()), peer.port());
        let slot = self.connections.take(peer.ip())?;
        let serving = Arc::clone(self);
        Some(async move { serving.connection(stream, peer, slot).await })
    }

    async fn connection(&self, stream: TcpStream, peer: SocketAddr, slot: Slot) {
        if !from_here(&stream, peer) {
            match self.edge.verdict(peer.ip(), CONNECTION).await {
                Verdict::Permitted => {}
                Verdict::Refused(written) => {
                    drop(stream);
                    return self.edge.write(written).await;
                }
                Verdict::Unavailable(error) => return error.log(),
            }
        }
        let Some(acceptor) = &self.tls else {
            return self
                .http(
                    Holding {
                        io: stream,
                        _slot: slot,
                    },
                    peer,
                )
                .await;
        };
        let Some(handshake) = self.handshakes.take(peer.ip()) else {
            return;
        };
        let accepted = tokio::time::timeout(self.limits.handshake_time, acceptor.accept(stream));
        // Failed handshakes are not logged: a scan would fill the log.
        let Ok(Ok(tls)) = accepted.await else {
            return;
        };
        drop(handshake);
        self.http(
            Holding {
                io: tls,
                _slot: slot,
            },
            peer,
        )
        .await;
    }

    /// Serves HTTP/1 on `io` with each request's peer as `ConnectInfo<SocketAddr>`, until the
    /// client closes, a deadline passes or the connection is upgraded. A client's protocol
    /// errors are not logged, as with handshakes.
    async fn http<T>(&self, io: Holding<T>, peer: SocketAddr)
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let router = self.router.clone();
        let service = service_fn(move |mut request: hyper::Request<Incoming>| {
            request.extensions_mut().insert(ConnectInfo(peer));
            router.clone().oneshot(request)
        });
        let _closed = http1::Builder::new()
            .timer(TokioTimer::new())
            .header_read_timeout(self.limits.header_time)
            .serve_connection(TokioIo::new(io), service)
            .with_upgrades()
            .await;
    }
}

/// Serves every connection `tcp` accepts until the returned future is dropped, which aborts
/// the connections still open.
pub(super) async fn run(tcp: TcpListener, serving: Arc<Serving>) -> Infallible {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            Some(_) = connections.join_next() => {}
            incoming = tcp.accept() => match incoming {
                Ok((stream, peer)) => {
                    if let Some(connection) = serving.admit(stream, peer) {
                        connections.spawn(connection);
                    }
                }
                Err(error) => accept_failed(&error).await,
            },
        }
    }
}

/// Whether `peer` is this host's own address the connection came in on, as for the compose
/// healthcheck run inside the container: it passes the accept-time check, and the layer then
/// lets it reach `GET /health` only (unless its address is permitted). No other container or
/// proxy address passes.
fn from_here(stream: &TcpStream, peer: SocketAddr) -> bool {
    stream
        .local_addr()
        .is_ok_and(|local| networks::canonical(local.ip()) == peer.ip())
}

/// A reset connection is the client's business; anything else is logged by kind and slows
/// the loop down, as axum's own TCP listener does.
async fn accept_failed(error: &io::Error) {
    if matches!(
        error.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    ) {
        return;
    }
    tracing::error!(domain = "access", fault = "accept", kind = %error.kind(), "accept failed");
    tokio::time::sleep(ACCEPT_PAUSE).await;
}

/// A connection and its place: an upgraded connection keeps the place until it closes.
struct Holding<T> {
    io: T,
    _slot: Slot,
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
        Pin::new(&mut self.get_mut().io).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().io).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.io.is_write_vectored()
    }
}
