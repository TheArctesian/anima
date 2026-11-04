//! Where and how the hub listens: only on loopback, private and VPN addresses (NET-003,
//! SEC-008), with TLS from the certificate files, reloaded when they change (SEC-001, D-13).

mod bind;
mod certificate;
mod health;
mod reload;
mod serve;
mod slots;
#[cfg(test)]
mod tests;
mod tls;

use certificate::Current;

use std::{convert::Infallible, io, net::IpAddr, net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::net::TcpListener;
use tokio_rustls::rustls::crypto::ring;

use super::Edge;
use crate::clock::Clock;
use serve::Serving;

use bind::check_bind;
pub use certificate::{Fault, TlsFiles};
pub use health::{HealthError, check_health};
pub use tls::TlsListener;

/// Why the hub cannot listen. Names addresses and fault kinds, never file content.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "ANIMA_LISTEN={0} is the unspecified address; give the VPN or private address to \
         listen on (NET-003)"
    )]
    Unspecified(IpAddr),
    #[error(
        "ANIMA_LISTEN={0} is not a loopback, private (10/8, 172.16/12, 192.168/16, fc00::/7) \
         or VPN (100.64/10) address; the hub must not listen on a public address (NET-003, \
         SEC-008)"
    )]
    Public(IpAddr),
    #[error(
        "ANIMA_TLS_CERT_FILE and ANIMA_TLS_KEY_FILE must be set to listen on {0}; plain HTTP \
         is only for loopback (SEC-001)"
    )]
    TlsRequired(IpAddr),
    #[error("the TLS certificate cannot be loaded: {0}")]
    Certificate(#[from] Fault),
    #[error("the TLS library refused the hub's settings")]
    TlsSetup,
    #[error("cannot listen on {addr} ({kind})")]
    Bind {
        addr: SocketAddr,
        kind: io::ErrorKind,
    },
}

/// A bound listener: plain on loopback without certificate files, else TLS.
pub enum Listening {
    Plain(TcpListener),
    Tls(TlsListener),
}

/// Checks `addr` (NET-003), loads the certificate if `tls` names the files, and binds. With
/// TLS, a task looks at the files every hour and swaps the certificate when they change
/// (R-021 H-2); it stops when the listener is dropped.
///
/// # Errors
///
/// The address or transport is refused, the certificate does not load, or the bind fails.
pub async fn listen(
    addr: SocketAddr,
    tls: Option<TlsFiles>,
    clock: Arc<dyn Clock>,
) -> Result<Listening, Error> {
    check_bind(addr.ip(), tls.is_some())?;
    let Some(files) = tls else {
        return Ok(Listening::Plain(bind(addr).await?));
    };
    let provider = Arc::new(ring::default_provider());
    let seen = reload::blocking(&files, reload::stamps).await?;
    let current = Arc::new(Current::new(reload::load(&files, &provider).await?));
    let config = tls::server_config(Arc::clone(&provider), Arc::clone(&current) as _)?;
    let tcp = bind(addr).await?;
    let state = reload::Reload::new(seen, clock.now());
    let served = reload::Served {
        files,
        current,
        provider,
    };
    let reloading = tokio::spawn(reload::run(state, served, clock));
    Ok(Listening::Tls(TlsListener::new(tcp, config, reloading)))
}

async fn bind(addr: SocketAddr) -> Result<TcpListener, Error> {
    TcpListener::bind(addr).await.map_err(|error| Error::Bind {
        addr,
        kind: error.kind(),
    })
}

impl Listening {
    /// The bound address (port 0 becomes the port the system chose).
    ///
    /// # Errors
    ///
    /// The socket cannot report it.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match self {
            Self::Plain(tcp) => tcp.local_addr(),
            Self::Tls(tls) => tls.tcp.local_addr(),
        }
    }

    /// Serves `router` until the returned future is dropped, with each request's TCP peer as
    /// `ConnectInfo<SocketAddr>` for the NET-002 layer. `edge` checks each peer at accept,
    /// before TLS, with the same list and refusal log as the layer.
    pub async fn serve(self, router: Router, edge: Edge) -> Infallible {
        self.serve_within(router, edge, serve::SERVED).await
    }

    async fn serve_within(self, router: Router, edge: Edge, limits: serve::Limits) -> Infallible {
        match self {
            Self::Plain(tcp) => serve::run(tcp, Serving::new(router, edge, None, limits)).await,
            Self::Tls(TlsListener {
                tcp,
                acceptor,
                reload: _reload,
            }) => serve::run(tcp, Serving::new(router, edge, Some(acceptor), limits)).await,
        }
    }
}
