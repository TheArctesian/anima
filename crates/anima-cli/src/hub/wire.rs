//! The byte stream both WebSocket clients run over (CMD-003): TCP to the origin's host, and TLS
//! over it for `https`, by rustls with ring and the web PKI roots. No proxy is used, and the
//! origin is `https`, or `http` on loopback only (CMD-013).

use std::sync::Arc;

use reqwest::Url;
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// A byte stream a socket runs over: TCP, or TLS over TCP.
pub trait Wire: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Wire for T {}

/// Connects to the host of `url`, with TLS when its scheme is `https`.
///
/// # Errors
/// The reason the connection or the handshake failed, without the URL.
pub async fn connect(url: &Url) -> Result<Box<dyn Wire>, String> {
    let port = url.port_or_known_default().ok_or("no port")?;
    let host = url.host_str().ok_or("no host")?;
    let address = host.trim_start_matches('[').trim_end_matches(']');
    let tcp = TcpStream::connect((address, port))
        .await
        .map_err(|error| error.to_string())?;
    if url.scheme() != "https" {
        return Ok(Box::new(tcp));
    }
    let name = ServerName::try_from(address.to_owned()).map_err(|_| "not a host name")?;
    let tls = tokio_rustls::TlsConnector::from(tls_config()?)
        .connect(name, tcp)
        .await
        .map_err(|error| error.to_string())?;
    Ok(Box::new(tls))
}

/// rustls with ring and the web PKI roots, HTTP/1.1 only (the upgrade needs it).
fn tls_config() -> Result<Arc<rustls::ClientConfig>, String> {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|_| "TLS could not be set up")?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}
