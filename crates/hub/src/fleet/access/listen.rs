//! Where and how the hub listens: only on loopback, private and VPN addresses (NET-003,
//! SEC-008), with TLS from the certificate files, reloaded when they change (SEC-001, D-13).

mod bind;
mod certificate;
mod health;
#[cfg_attr(windows, path = "listen/link/windows.rs")]
mod link;
mod reload;
mod serve;
mod slots;
#[cfg(test)]
mod tests;
mod timed;
mod tls;

use certificate::Current;

use std::{io, net::IpAddr, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use axum::Router;
use tokio::net::TcpListener;
use tokio_rustls::rustls::crypto::ring;

use super::Edge;
use crate::clock::Clock;
use serve::Serving;

use bind::check_bind;
pub use certificate::{Fault, TlsFiles};
pub use health::{HealthError, check_health};
pub use link::{LinkSocket, link_socket, own_daemon_socket};
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
    #[error(
        "ANIMA_LINK_SOCKET={0}: its folder must be a folder of the hub's user that no other \
         user can reach (mode 0700)"
    )]
    LinkFolder(PathBuf),
    #[error("ANIMA_LINK_SOCKET={0} is a file that is not a socket; the hub does not replace it")]
    LinkPath(PathBuf),
    #[error("cannot make the link's socket {path} ({kind})")]
    LinkSocket { path: PathBuf, kind: io::ErrorKind },
    #[error(
        "ANIMA_OWN_DAEMON_SOCKET={0}: its folder must be a folder of the hub's user that no \
         other user can reach (mode 0700)"
    )]
    OwnDaemonFolder(PathBuf),
    #[error(
        "ANIMA_OWN_DAEMON_SOCKET={0} is a file that is not a socket; the hub does not replace it"
    )]
    OwnDaemonPath(PathBuf),
    #[error("cannot make the socket of the hub's own daemon {path} ({kind})")]
    OwnDaemonSocket { path: PathBuf, kind: io::ErrorKind },
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

    /// This listener with `edge` checking each peer at accept, before TLS, with the same list
    /// and refusal log as the layer (NET-002).
    #[must_use]
    pub fn admitting(self, edge: Edge) -> Entrance {
        Entrance {
            listening: self,
            edge,
            link: None,
            own_daemon: None,
        }
    }
}

/// A listener and the edge that checks its peers: ready to serve.
pub struct Entrance {
    listening: Listening,
    edge: Edge,
    link: Option<LinkSocket>,
    own_daemon: Option<LinkSocket>,
}

impl Entrance {
    /// This entrance with the link's socket as a second way in, in the link mode. Its
    /// connections are served as the listener's are, each with the device the link names as
    /// its peer.
    #[must_use]
    pub fn with_link(self, link: Option<LinkSocket>) -> Self {
        Self { link, ..self }
    }

    /// This entrance with the socket of the hub's own daemon as a way in, in the hosted mode
    /// (HMD-071, HM-T35). Its connections reach only the routes of K8, each as from this
    /// computer, which is the hub's own device.
    #[must_use]
    pub fn with_own_daemon(self, own_daemon: Option<LinkSocket>) -> Self {
        Self { own_daemon, ..self }
    }

    /// Serves `router` until `stop` resolves, with each request's TCP peer as
    /// `ConnectInfo<SocketAddr>` for the NET-002 layer. Then stops accepting, lets open
    /// requests finish for at most `drain_limit`, and drops the rest (R-040).
    pub async fn serve_until(
        self,
        router: Router,
        stop: impl Future<Output = ()>,
        drain_limit: Duration,
    ) {
        let Self {
            listening,
            edge,
            link,
            own_daemon,
        } = self;
        let (tcp, acceptor, _reload) = match listening {
            Listening::Plain(tcp) => (tcp, None, None),
            Listening::Tls(TlsListener {
                tcp,
                acceptor,
                reload,
            }) => (tcp, Some(acceptor), Some(reload)),
        };
        let serving = Serving::new(router, edge, acceptor, serve::SERVED);
        serve::run(
            serve::Ways {
                tcp,
                link,
                own_daemon,
            },
            serving,
            stop,
            drain_limit,
        )
        .await;
    }
}
