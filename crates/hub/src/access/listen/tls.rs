//! A TCP listener that hands axum only connections whose TLS handshake finished (SEC-001).
//!
//! Handshakes run in their own tasks with a deadline and a cap, so a client that stalls its
//! handshake holds neither the accept loop nor more than one slot.

use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use axum::serve::Listener;
use tokio::{
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{ServerConfig, server::ResolvesServerCert, version::TLS13},
    server::TlsStream,
};

use super::Error;

/// Handshakes in progress at once; further connections wait in the kernel's backlog.
const HANDSHAKES: usize = 256;

/// A handshake that takes longer is dropped.
const HANDSHAKE_TIME: Duration = Duration::from_secs(10);

/// After an accept error other than a reset connection (for example too many open files).
const ACCEPT_PAUSE: Duration = Duration::from_secs(1);

/// TLS 1.3 only, no client certificates, the certificate from `certificates`.
pub(super) fn server_config(
    provider: Arc<tokio_rustls::rustls::crypto::CryptoProvider>,
    certificates: Arc<dyn ResolvesServerCert>,
) -> Result<Arc<ServerConfig>, Error> {
    let config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&TLS13])
        .map_err(|_| Error::TlsSetup)?
        .with_no_client_auth()
        .with_cert_resolver(certificates);
    Ok(Arc::new(config))
}

/// A connection after its handshake, and its TCP peer.
type Accepted = (TlsStream<TcpStream>, SocketAddr);

pub struct TlsListener {
    tcp: TcpListener,
    acceptor: TlsAcceptor,
    handshakes: JoinSet<Option<Accepted>>,
    /// The certificate reload; stopped when the listener is dropped.
    reload: JoinHandle<()>,
}

impl TlsListener {
    pub(super) fn new(tcp: TcpListener, config: Arc<ServerConfig>, reload: JoinHandle<()>) -> Self {
        Self {
            tcp,
            acceptor: TlsAcceptor::from(config),
            handshakes: JoinSet::new(),
            reload,
        }
    }
}

impl Drop for TlsListener {
    fn drop(&mut self) {
        self.reload.abort();
    }
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> Accepted {
        loop {
            let room = self.handshakes.len() < HANDSHAKES;
            tokio::select! {
                Some(done) = self.handshakes.join_next() => {
                    if let Ok(Some(accepted)) = done {
                        return accepted;
                    }
                }
                incoming = self.tcp.accept(), if room => match incoming {
                    Ok((stream, peer)) => {
                        self.handshakes.spawn(handshake(self.acceptor.clone(), stream, peer));
                    }
                    Err(error) => accept_failed(&error).await,
                },
            }
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.tcp.local_addr()
    }
}

/// The connection after its handshake, or `None` when it failed or ran out of time. Failed
/// handshakes are not logged: a scan would fill the log.
async fn handshake(acceptor: TlsAcceptor, stream: TcpStream, peer: SocketAddr) -> Option<Accepted> {
    match tokio::time::timeout(HANDSHAKE_TIME, acceptor.accept(stream)).await {
        Ok(Ok(tls)) => Some((tls, peer)),
        _ => None,
    }
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
