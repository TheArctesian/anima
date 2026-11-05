//! The TLS side of a listener (SEC-001): TLS 1.3 only, the served certificate from the
//! reloading resolver. Handshakes run per connection in `serve`, after the peer check.

use std::sync::Arc;

use tokio::{net::TcpListener, task::JoinHandle};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{ServerConfig, server::ResolvesServerCert, version::TLS13},
};

use super::Error;

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

/// A bound TCP listener whose connections get a TLS handshake.
pub struct TlsListener {
    pub(super) tcp: TcpListener,
    pub(super) acceptor: TlsAcceptor,
    pub(super) reload: Reloading,
}

/// The certificate reload task; stopped when dropped.
pub(super) struct Reloading(JoinHandle<()>);

impl Drop for Reloading {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl TlsListener {
    pub(super) fn new(tcp: TcpListener, config: Arc<ServerConfig>, reload: JoinHandle<()>) -> Self {
        Self {
            tcp,
            acceptor: TlsAcceptor::from(config),
            reload: Reloading(reload),
        }
    }
}
