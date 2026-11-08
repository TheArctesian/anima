//! Local servers for the egress tests. Each records the connections it accepts, the request or
//! command lines that reach it and the TLS server name each handshake asked for.

use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use rustls::{
    ServerConfig,
    crypto::ring,
    pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::TlsAcceptor;

/// How a server answers each connection.
#[derive(Clone)]
pub(super) enum Behaviour {
    /// HTTP: read one request, write these bytes, close; over TLS when an acceptor is given.
    Http(Vec<u8>, Option<TlsAcceptor>),
    /// HTTP: read one request, answer 200 with a body that never ends.
    Endless,
    /// SMTP over TLS from the first byte.
    SmtpImplicit(TlsAcceptor),
    /// Plain SMTP that offers no STARTTLS.
    SmtpPlain,
    /// SMTP that offers STARTTLS and upgrades with this acceptor.
    SmtpStartTls(TlsAcceptor),
}

/// What a server saw.
#[derive(Default)]
struct Seen {
    peers: Mutex<Vec<SocketAddr>>,
    ended: AtomicUsize,
    lines: Mutex<Vec<String>>,
    names: Mutex<Vec<String>>,
    written: AtomicUsize,
}

pub(super) struct Server {
    pub(super) port: u16,
    seen: Arc<Seen>,
    sentinels: Mutex<Vec<SocketAddr>>,
}

impl Server {
    /// Waits until the server has accepted every connection made before this call and ended
    /// each, so what it saw is complete: a sentinel connection is accepted after them.
    pub(super) async fn settle(&self) {
        let sentinel = TcpStream::connect(("127.0.0.1", self.port)).await.unwrap();
        let address = sentinel.local_addr().unwrap();
        drop(sentinel);
        self.sentinels.lock().unwrap().push(address);
        let seen = self.seen.clone();
        let done = async move {
            loop {
                let peers = seen.peers.lock().unwrap().clone();
                if peers.contains(&address) && seen.ended.load(Ordering::SeqCst) == peers.len() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(60), done)
            .await
            .expect("the test server settles");
    }

    /// The connections accepted, not counting [`Server::settle`]'s own.
    pub(super) fn connections(&self) -> usize {
        let sentinels = self.sentinels.lock().unwrap();
        let peers = self.seen.peers.lock().unwrap();
        peers
            .iter()
            .filter(|peer| !sentinels.contains(peer))
            .count()
    }

    /// The HTTP request lines or SMTP command lines received, in order.
    pub(super) fn lines(&self) -> Vec<String> {
        self.seen.lines.lock().unwrap().clone()
    }

    /// The server name of each completed TLS handshake ("" when the client sent none).
    pub(super) fn server_names(&self) -> Vec<String> {
        self.seen.names.lock().unwrap().clone()
    }

    /// The body bytes an [`Behaviour::Endless`] server wrote before the client went.
    pub(super) fn written(&self) -> usize {
        self.seen.written.load(Ordering::SeqCst)
    }
}

/// A self-signed certificate for `name`: its DER and a TLS server presenting it.
pub(super) fn certificate(name: &str) -> (Vec<u8>, TlsAcceptor) {
    let key = rcgen::KeyPair::generate().unwrap();
    let params = rcgen::CertificateParams::new(vec![name.to_owned()]).unwrap();
    let der = params.self_signed(&key).unwrap().der().to_vec();
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key.serialize_der()));
    let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![der.clone().into()], key)
        .unwrap();
    (der, TlsAcceptor::from(Arc::new(config)))
}

/// A server on `127.0.0.1` that answers each connection as `behaviour` says.
pub(super) async fn serve(behaviour: Behaviour) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Seen::default());
    let recorder = seen.clone();
    tokio::spawn(async move {
        while let Ok((stream, peer)) = listener.accept().await {
            recorder.peers.lock().unwrap().push(peer);
            let (seen, behaviour) = (recorder.clone(), behaviour.clone());
            tokio::spawn(async move {
                answer(stream, behaviour, &seen).await;
                seen.ended.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    Server {
        port,
        seen,
        sentinels: Mutex::new(Vec::new()),
    }
}

async fn answer(stream: TcpStream, behaviour: Behaviour, seen: &Seen) {
    match behaviour {
        Behaviour::Http(reply, None) => http(stream, &reply, seen).await,
        Behaviour::Http(reply, Some(acceptor)) => {
            if let Some(tls) = handshake(&acceptor, stream, seen).await {
                http(tls, &reply, seen).await;
            }
        }
        Behaviour::Endless => endless(stream, seen).await,
        Behaviour::SmtpPlain => drop(smtp(stream, true, false, seen).await),
        Behaviour::SmtpImplicit(acceptor) => {
            if let Some(tls) = handshake(&acceptor, stream, seen).await {
                smtp(tls, true, false, seen).await;
            }
        }
        Behaviour::SmtpStartTls(acceptor) => {
            let Some(plain) = smtp(stream, true, true, seen).await else {
                return;
            };
            if let Some(tls) = handshake(&acceptor, plain, seen).await {
                smtp(tls, false, false, seen).await;
            }
        }
    }
}

async fn handshake(
    acceptor: &TlsAcceptor,
    stream: TcpStream,
    seen: &Seen,
) -> Option<tokio_rustls::server::TlsStream<TcpStream>> {
    let tls = acceptor.accept(stream).await.ok()?;
    let name = tls.get_ref().1.server_name().unwrap_or("").to_owned();
    seen.names.lock().unwrap().push(name);
    Some(tls)
}

/// Reads one request (head and `Content-Length` body), records its request line, returns the
/// reader.
async fn request<S: AsyncRead + Unpin>(stream: S, seen: &Seen) -> Option<BufReader<S>> {
    let mut reader = BufReader::new(stream);
    let mut length = 0;
    let mut first = true;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.ok()? == 0 {
            return None;
        }
        if line == "\r\n" {
            break;
        }
        if first {
            seen.lines.lock().unwrap().push(line.trim_end().to_owned());
            first = false;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).await.ok()?;
    Some(reader)
}

async fn http<S: AsyncRead + AsyncWrite + Unpin>(stream: S, reply: &[u8], seen: &Seen) {
    if let Some(mut reader) = request(stream, seen).await {
        let _ = reader.get_mut().write_all(reply).await;
        let _ = reader.get_mut().shutdown().await;
    }
}

async fn endless(stream: TcpStream, seen: &Seen) {
    let Some(mut reader) = request(stream, seen).await else {
        return;
    };
    let head = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n";
    if reader.get_mut().write_all(head).await.is_err() {
        return;
    }
    let chunk = [b'x'; 1024];
    while reader.get_mut().write_all(&chunk).await.is_ok() {
        seen.written.fetch_add(chunk.len(), Ordering::SeqCst);
    }
}

/// One SMTP conversation. Records every command line but the message content; returns the
/// stream when the client asks for STARTTLS.
async fn smtp<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    greet: bool,
    offer_starttls: bool,
    seen: &Seen,
) -> Option<S> {
    let mut reader = BufReader::new(stream);
    if greet {
        reader
            .get_mut()
            .write_all(b"220 test ESMTP\r\n")
            .await
            .ok()?;
    }
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.ok()? == 0 {
            return None;
        }
        seen.lines.lock().unwrap().push(line.trim_end().to_owned());
        let verb = line.get(..4).unwrap_or_default().to_ascii_uppercase();
        let reply: &[u8] = match verb.as_str() {
            "EHLO" if offer_starttls => b"250-test\r\n250-STARTTLS\r\n250 AUTH PLAIN LOGIN\r\n",
            "EHLO" => b"250-test\r\n250 AUTH PLAIN LOGIN\r\n",
            "STAR" => {
                reader.get_mut().write_all(b"220 go ahead\r\n").await.ok()?;
                return Some(reader.into_inner());
            }
            "AUTH" => b"235 accepted\r\n",
            "MAIL" | "RCPT" => b"250 ok\r\n",
            "DATA" => {
                reader.get_mut().write_all(b"354 go on\r\n").await.ok()?;
                skip_content(&mut reader).await?;
                b"250 queued\r\n"
            }
            "QUIT" => {
                let _ = reader.get_mut().write_all(b"221 bye\r\n").await;
                return None;
            }
            _ => b"500 unknown\r\n",
        };
        reader.get_mut().write_all(reply).await.ok()?;
    }
}

/// Reads the message content up to the line holding a single dot.
async fn skip_content<S: AsyncRead + Unpin>(reader: &mut BufReader<S>) -> Option<()> {
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.ok()? == 0 {
            return None;
        }
        if line == ".\r\n" {
            return Some(());
        }
    }
}
