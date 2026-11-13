//! The way in for every connection (NET-002, SEC-001): the peer is checked against the
//! permitted networks at TCP accept, before any TLS work, and a refused peer is closed at once.
//! Caps in total and per peer bound connections and handshakes; deadlines bound the handshake,
//! every request head (including the wait for the next request on a kept-alive connection),
//! every request body, every answer and every stalled write.
//!
//! The accept loop never waits on a client: each connection runs in its own task. On the stop
//! signal the loop stops accepting, asks every connection to finish its request and close,
//! waits at most the drain limit, then aborts what is left (R-040).

use std::{convert::Infallible, io, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Router,
    extract::ConnectInfo,
    http::StatusCode,
    response::{IntoResponse as _, Response},
};
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    sync::watch,
    task::JoinSet,
};
use tokio_rustls::TlsAcceptor;
use tower::ServiceExt as _;

use super::{
    slots::{Slot, Slots},
    timed::{Holding, timed_body},
};
use crate::fleet::access::{
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
    /// A request body must arrive whole within this, from its head.
    pub body_time: Duration,
    /// A route that has not answered within this gets 503 in its place.
    pub answer_time: Duration,
    /// A write that makes no progress for this closes the connection.
    pub write_stall: Duration,
}

/// The limits the hub serves with.
pub(super) const SERVED: Limits = Limits {
    connections: 1024,
    connections_per_peer: 64,
    handshakes_per_peer: 8,
    handshake_time: Duration::from_secs(5),
    header_time: Duration::from_secs(10),
    body_time: Duration::from_secs(30),
    answer_time: Duration::from_mins(1),
    write_stall: Duration::from_secs(30),
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
    /// Set once serving stops: open connections finish their request and close.
    stopping: watch::Sender<bool>,
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
            stopping: watch::Sender::new(false),
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
                Verdict::Unavailable(error) => return self.edge.unavailable(&error),
            }
        }
        let stall = self.limits.write_stall;
        let Some(acceptor) = &self.tls else {
            return self.http(Holding::new(stream, slot, stall), peer).await;
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
        self.http(Holding::new(tls, slot, stall), peer).await;
    }

    /// Serves HTTP/1 on `io` with each request's peer as `ConnectInfo<SocketAddr>`, until the
    /// client closes, a deadline passes, the connection is upgraded or serving stops. A
    /// client's protocol errors are not logged, as with handshakes.
    async fn http<T>(&self, io: Holding<T>, peer: SocketAddr)
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (router, limits) = (self.router.clone(), self.limits);
        let service = service_fn(move |mut request: hyper::Request<Incoming>| {
            request.extensions_mut().insert(ConnectInfo(peer));
            let request = request.map(|body| timed_body(body, limits.body_time));
            let answer = tokio::time::timeout(limits.answer_time, router.clone().oneshot(request));
            async move { answer.await.unwrap_or_else(|_| Ok::<_, Infallible>(late())) }
        });
        let connection = http1::Builder::new()
            .timer(TokioTimer::new())
            .header_read_timeout(limits.header_time)
            .serve_connection(TokioIo::new(io), service)
            .with_upgrades();
        tokio::pin!(connection);
        let mut stopping = self.stopping.subscribe();
        tokio::select! {
            _closed = connection.as_mut() => return,
            _stop = stopping.wait_for(|stop| *stop) => {}
        }
        connection.as_mut().graceful_shutdown();
        let _closed = connection.await;
    }
}

/// The answer in place of one that missed [`Limits::answer_time`].
fn late() -> Response {
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}

/// Serves every connection `tcp` accepts until `stop` resolves; then stops accepting and
/// drains the open connections for at most `drain_limit` before aborting the rest (R-040).
pub(super) async fn run(
    tcp: TcpListener,
    serving: Arc<Serving>,
    stop: impl Future<Output = ()>,
    drain_limit: Duration,
) {
    let mut connections = JoinSet::new();
    tokio::pin!(stop);
    loop {
        tokio::select! {
            () = &mut stop => break,
            Some(_) = connections.join_next() => {}
            incoming = tcp.accept() => match incoming {
                Ok((stream, peer)) => connections.extend(serving.admit(stream, peer)),
                Err(error) => accept_failed(&error).await,
            },
        }
    }
    drop(tcp);
    serving.stopping.send_replace(true);
    drain(connections, drain_limit).await;
}

/// Waits at most `limit` for `connections` to finish; the rest are aborted when dropped.
async fn drain(mut connections: JoinSet<()>, limit: Duration) {
    let drained = async { while connections.join_next().await.is_some() {} };
    if tokio::time::timeout(limit, drained).await.is_err() {
        tracing::warn!("drain limit reached; open requests are dropped");
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
