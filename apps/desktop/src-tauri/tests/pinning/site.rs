//! A local HTTPS site with a fresh self-signed certificate, on loopback only. Every request is
//! its own connection, so every load is a new TLS handshake, and the site counts the requests
//! it answered: a cancelled challenge sends none.

use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use rustls::{
    ServerConfig,
    crypto::ring,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
    task::{JoinHandle, JoinSet},
};
use tokio_rustls::TlsAcceptor;

use crate::web_view::Fingerprint;

/// Most request bytes read before the headers end; a longer request is dropped.
const REQUEST_LIMIT: usize = 16 * 1024;
/// Most connections open at once; more are closed on accept.
const CONNECTIONS: usize = 64;
const BODY: &str = "<!doctype html><title>proof</title><p>loaded</p>";

/// A running site. Dropping it does not stop it; call `stop`.
pub struct Site {
    pub port: u16,
    /// SHA-256 of its leaf certificate.
    pub leaf: Fingerprint,
    served: Arc<AtomicUsize>,
    accept: JoinHandle<()>,
}

impl Site {
    /// Serves on `address` and `port` (0 for any) with a new self-signed certificate, or with
    /// a new leaf signed by a new issuer, presented as the chain leaf then issuer.
    pub fn serve(address: IpAddr, port: u16, chained: bool) -> Result<Self, String> {
        let (chain, key) = if chained {
            issued_chain()
        } else {
            self_signed()
        }?;
        let leaf = Fingerprint::of_certificate(&chain[0]);
        let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|error| error.to_string())?
            .with_no_client_auth()
            .with_single_cert(chain, key)
            .map_err(|error| error.to_string())?;
        let listener = bind(SocketAddr::new(address, port)).map_err(|error| error.to_string())?;
        let port = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        let served = Arc::new(AtomicUsize::new(0));
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let accept = tokio::spawn(accept_loop(listener, acceptor, served.clone()));
        Ok(Self {
            port,
            leaf,
            served,
            accept,
        })
    }

    /// How many requests it has answered.
    pub fn served(&self) -> usize {
        self.served.load(Ordering::SeqCst)
    }

    /// Stops accepting and closes every open connection, which frees the port.
    pub async fn stop(self) {
        self.accept.abort();
        // An aborted task answers `JoinError::Cancelled`: the expected result.
        let _cancelled = self.accept.await;
    }
}

type Credentials = (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>);

fn self_signed() -> Result<Credentials, String> {
    let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
        .map_err(|error| error.to_string())?;
    let key = PrivatePkcs8KeyDer::from(issued.signing_key.serialize_der());
    Ok((vec![issued.cert.der().clone()], key.into()))
}

fn issued_chain() -> Result<Credentials, String> {
    let text = |error: rcgen::Error| error.to_string();
    let issuer_key = rcgen::KeyPair::generate().map_err(text)?;
    let mut issuer_params = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(text)?;
    issuer_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let issuer = issuer_params.self_signed(&issuer_key).map_err(text)?;
    let leaf_key = rcgen::KeyPair::generate().map_err(text)?;
    let leaf = rcgen::CertificateParams::new(vec!["localhost".to_owned()])
        .map_err(text)?
        .signed_by(
            &leaf_key,
            &rcgen::Issuer::from_params(&issuer_params, &issuer_key),
        )
        .map_err(text)?;
    let key = PrivatePkcs8KeyDer::from(leaf_key.serialize_der());
    Ok((vec![leaf.der().clone(), issuer.der().clone()], key.into()))
}

/// With address reuse, so a restarted site gets the same port.
fn bind(address: SocketAddr) -> io::Result<TcpListener> {
    let socket = if address.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };
    socket.set_reuseaddr(true)?;
    socket.bind(address)?;
    socket.listen(32)
}

async fn accept_loop(listener: TcpListener, acceptor: TlsAcceptor, served: Arc<AtomicUsize>) {
    // Owned here, so aborting this task drops, and so ends, every connection.
    let mut connections = JoinSet::new();
    loop {
        let Ok((stream, _peer)) = listener.accept().await else {
            continue;
        };
        while connections.try_join_next().is_some() {}
        if connections.len() < CONNECTIONS {
            connections.spawn(answer(stream, acceptor.clone(), served.clone()));
        }
    }
}

/// One connection: the handshake, one request, one page, then close. A refused handshake is
/// what a cancelled challenge looks like from here.
async fn answer(stream: TcpStream, acceptor: TlsAcceptor, served: Arc<AtomicUsize>) {
    let Ok(mut tls) = acceptor.accept(stream).await else {
        return;
    };
    let mut request = Vec::new();
    let mut chunk = [0; 1024];
    while !request.windows(4).any(|end| end == b"\r\n\r\n") {
        match tls.read(&mut chunk).await {
            Ok(read) if read > 0 && request.len() + read <= REQUEST_LIMIT => {
                request.extend_from_slice(&chunk[..read]);
            }
            _ => return,
        }
    }
    served.fetch_add(1, Ordering::SeqCst);
    let page = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{BODY}",
        BODY.len()
    );
    // The page is the whole answer; a client that has gone changes nothing for the proof.
    let _gone = tls.write_all(page.as_bytes()).await;
    let _closed = tls.shutdown().await;
}
