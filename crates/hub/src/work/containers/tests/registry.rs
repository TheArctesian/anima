//! A local HTTPS server that stands in for every registry (HOST-021): it answers each request
//! by its method and path, and records the head of each request. Nothing reaches the internet.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, PoisonError},
};

use rustls::{
    ServerConfig,
    crypto::ring,
    pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};
use tokio_rustls::TlsAcceptor;

use crate::work::containers::updates::egress::Egress;

/// The token the stand-in gives.
pub(super) const TOKEN: &str = "anonymous-reader-token";

pub(super) struct Registry {
    port: u16,
    root: Vec<u8>,
    heads: Arc<Mutex<Vec<String>>>,
}

impl Registry {
    /// The rules of a connection that reaches this server in place of every registry.
    pub(super) fn egress(&self) -> Egress {
        let origin = format!("https://localhost:{}/", self.port).parse().unwrap();
        Egress::new(crate::fleet::ssh::tests::reaching()).standing_in(origin, self.root.clone())
    }

    pub(super) const fn port(&self) -> u16 {
        self.port
    }

    /// The head of each request received, in order: the request line, then its header lines.
    pub(super) fn heads(&self) -> Vec<String> {
        let heads = self.heads.lock();
        heads.unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The request lines received, sorted.
    pub(super) fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .heads()
            .iter()
            .map(|head| head.lines().next().unwrap_or_default().to_owned())
            .collect();
        lines.sort();
        lines
    }
}

/// An answer with this status line, the header lines `headers` (each ends with a line break)
/// and no body.
pub(super) fn answer(status: &str, headers: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n{headers}\r\n")
        .into_bytes()
}

/// The answer of a token endpoint.
pub(super) fn token() -> Vec<u8> {
    let body = format!(r#"{{"token":"{TOKEN}","expires_in":300}}"#);
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// A manifest answer that gives `digest`.
pub(super) fn manifest(digest: &str) -> Vec<u8> {
    answer("200 OK", &format!("Docker-Content-Digest: {digest}\r\n"))
}

/// A server on 127.0.0.1 that answers a request with `reply(method, path)`.
pub(super) async fn serve(
    reply: impl Fn(&str, &str) -> Vec<u8> + Send + Sync + 'static,
) -> Registry {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let key = rcgen::KeyPair::generate().unwrap();
    let params = rcgen::CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
    let root = params.self_signed(&key).unwrap().der().to_vec();
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key.serialize_der()));
    let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![root.clone().into()], key)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let heads = Arc::new(Mutex::new(Vec::new()));
    let (seen, reply) = (Arc::clone(&heads), Arc::new(reply));
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let (acceptor, seen, reply) = (acceptor.clone(), Arc::clone(&seen), Arc::clone(&reply));
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(stream).await {
                    respond(tls, &*reply, &seen).await;
                }
            });
        }
    });
    Registry { port, root, heads }
}

/// Reads one request head, records it and writes the answer for its method and path.
async fn respond(
    stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    reply: &(dyn Fn(&str, &str) -> Vec<u8> + Send + Sync),
    seen: &Mutex<Vec<String>>,
) {
    let mut lines = BufReader::new(stream).lines();
    let mut head = String::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.is_empty() {
            break;
        }
        head.push_str(&line);
        head.push('\n');
    }
    let mut words = head.split_whitespace();
    let (method, path) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    let bytes = reply(method, path);
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(head.clone());
    let mut stream = lines.into_inner().into_inner();
    let _ = stream.write_all(&bytes).await;
    let _ = stream.shutdown().await;
}
