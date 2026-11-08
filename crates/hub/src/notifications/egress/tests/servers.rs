//! Local servers for the egress tests. Each records the connections it accepts, the request or
//! command lines that reach it, the HTTP request headers and bodies, and the TLS server name
//! each handshake asked for.

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
    /// Read whatever arrives and never answer, until the client goes.
    Silent,
    /// SMTP over TLS from the first byte.
    SmtpImplicit(TlsAcceptor),
    /// SMTP over TLS from the first byte that refuses every login with 535.
    SmtpRefusingLogin(TlsAcceptor),
    /// Plain SMTP that offers no STARTTLS.
    SmtpPlain,
    /// SMTP that offers STARTTLS and upgrades with this acceptor.
    SmtpStartTls(TlsAcceptor),
    /// SMTP that offers STARTTLS and writes a plaintext 554 right after its 220 answer, in the
    /// same write, then upgrades with this acceptor (a STARTTLS response injection).
    SmtpStartTlsInjecting(TlsAcceptor),
}

/// How an SMTP server answers.
#[derive(Clone, Copy)]
struct Script {
    /// Whether it writes the 220 greeting first (not after STARTTLS).
    greet: bool,
    /// What it answers to STARTTLS; `None` when it does not offer it.
    starttls: Option<&'static [u8]>,
    /// What it answers to AUTH.
    auth: &'static [u8],
}

const GO_AHEAD: &[u8] = b"220 go ahead\r\n";
/// A plain conversation with no STARTTLS and every login accepted.
const PLAIN: Script = Script {
    greet: true,
    starttls: None,
    auth: b"235 accepted\r\n",
};
/// The conversation after STARTTLS: no new greeting, no second STARTTLS.
const AFTER_STARTTLS: Script = Script {
    greet: false,
    ..PLAIN
};

/// What a server saw.
#[derive(Default)]
struct Seen {
    peers: Mutex<Vec<SocketAddr>>,
    ended: AtomicUsize,
    lines: Mutex<Vec<String>>,
    headers: Mutex<Vec<String>>,
    bodies: Mutex<Vec<Vec<u8>>>,
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

    /// The HTTP header lines received, in order, across all requests.
    pub(super) fn headers(&self) -> Vec<String> {
        self.seen.headers.lock().unwrap().clone()
    }

    /// The HTTP request bodies received, in order.
    pub(super) fn bodies(&self) -> Vec<Vec<u8>> {
        self.seen.bodies.lock().unwrap().clone()
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
        Behaviour::Silent => silent(stream).await,
        Behaviour::SmtpPlain => drop(smtp(stream, PLAIN, seen).await),
        Behaviour::SmtpImplicit(acceptor) => implicit(&acceptor, stream, PLAIN, seen).await,
        Behaviour::SmtpRefusingLogin(acceptor) => {
            let script = Script {
                auth: b"535 refused\r\n",
                ..PLAIN
            };
            implicit(&acceptor, stream, script, seen).await;
        }
        Behaviour::SmtpStartTls(acceptor) => starttls(&acceptor, stream, GO_AHEAD, seen).await,
        Behaviour::SmtpStartTlsInjecting(acceptor) => {
            let injected = b"220 go ahead\r\n554 injected before TLS\r\n";
            starttls(&acceptor, stream, injected, seen).await;
        }
    }
}

/// SMTP over TLS from the first byte, answering as `script` says.
async fn implicit(acceptor: &TlsAcceptor, stream: TcpStream, script: Script, seen: &Seen) {
    if let Some(tls) = handshake(acceptor, stream, seen).await {
        smtp(tls, script, seen).await;
    }
}

/// SMTP that offers STARTTLS, answers it with `go_ahead` and upgrades.
async fn starttls(acceptor: &TlsAcceptor, stream: TcpStream, go_ahead: &'static [u8], seen: &Seen) {
    let script = Script {
        starttls: Some(go_ahead),
        ..PLAIN
    };
    let Some(plain) = smtp(stream, script, seen).await else {
        return;
    };
    if let Some(tls) = handshake(acceptor, plain, seen).await {
        smtp(tls, AFTER_STARTTLS, seen).await;
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

/// Reads one request (head and `Content-Length` body), records its request line, header lines
/// and body, returns the reader.
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
        let recorded = if first { &seen.lines } else { &seen.headers };
        recorded.lock().unwrap().push(line.trim_end().to_owned());
        first = false;
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).await.ok()?;
    seen.bodies.lock().unwrap().push(body);
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

/// Reads and drops whatever arrives until the client closes.
async fn silent(mut stream: TcpStream) {
    let mut sink = [0; 1024];
    while stream.read(&mut sink).await.is_ok_and(|read| read > 0) {}
}

/// One SMTP conversation answered as `script` says. Records every command line but the message
/// content; returns the stream when the client asks for STARTTLS.
async fn smtp<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    script: Script,
    seen: &Seen,
) -> Option<S> {
    let mut reader = BufReader::new(stream);
    if script.greet {
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
        let reply: &[u8] = match (verb.as_str(), script.starttls) {
            ("EHLO", Some(_)) => b"250-test\r\n250-STARTTLS\r\n250 AUTH PLAIN LOGIN\r\n",
            ("EHLO", None) => b"250-test\r\n250 AUTH PLAIN LOGIN\r\n",
            ("STAR", Some(go_ahead)) => {
                reader.get_mut().write_all(go_ahead).await.ok()?;
                return Some(reader.into_inner());
            }
            ("AUTH", _) => script.auth,
            ("MAIL" | "RCPT", _) => b"250 ok\r\n",
            ("DATA", _) => {
                reader.get_mut().write_all(b"354 go on\r\n").await.ok()?;
                skip_content(&mut reader).await?;
                b"250 queued\r\n"
            }
            ("QUIT", _) => {
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
