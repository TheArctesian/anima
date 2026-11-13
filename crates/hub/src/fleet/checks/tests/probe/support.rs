//! Local servers and environments that the probe tests share.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use chrono::{DateTime, TimeZone, Utc};
use rustls::{ServerConfig, crypto::ring, pki_types::PrivatePkcs8KeyDer};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, UdpSocket},
};
use tokio_rustls::TlsAcceptor;

use crate::fleet::{
    checks::probe::{ProbeEnv, ProbeTarget},
    targets::AddressPolicy,
};

pub const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

pub fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 4, 12, 0, 0).unwrap()
}

/// An environment that may reach loopback, where the test servers listen.
pub fn env() -> ProbeEnv {
    ProbeEnv::new(AddressPolicy::default().allowing_loopback(), now())
}

/// The production default policy.
pub fn strict_env() -> ProbeEnv {
    ProbeEnv::new(AddressPolicy::default(), now())
}

pub fn target(host: &str) -> ProbeTarget {
    ProbeTarget {
        host: host.to_owned(),
    }
}

/// A request as the test server saw it.
#[derive(Clone, Debug)]
pub struct Request {
    pub path: String,
    pub headers: Vec<(String, String)>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub enum Body {
    Bytes(Vec<u8>),
    /// `len` bytes made of `prefix` then `fill`; `written` counts what reached the socket.
    Stream {
        prefix: Vec<u8>,
        fill: u8,
        len: usize,
        written: Arc<AtomicUsize>,
    },
    /// Accepts the request and never answers.
    Hang,
}

pub struct Reply {
    pub head: String,
    pub body: Body,
}

impl Reply {
    pub fn ok(body: &str) -> Self {
        Self::status(200, body)
    }

    pub fn status(code: u16, body: &str) -> Self {
        let head = format!(
            "HTTP/1.1 {code} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        Self {
            head,
            body: Body::Bytes(body.as_bytes().to_vec()),
        }
    }

    pub fn redirect(location: &str) -> Self {
        let head = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        Self {
            head,
            body: Body::Bytes(Vec::new()),
        }
    }
}

pub struct Server {
    pub addr: SocketAddr,
    pub requests: Arc<Mutex<Vec<Request>>>,
    pub connections: Arc<AtomicUsize>,
}

impl Server {
    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.addr.port())
    }

    pub fn seen(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

async fn read_request<S: AsyncRead + Unpin>(stream: &mut S) -> Option<Request> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).await.ok()?;
        head.push(byte[0]);
        if head.len() > 64 * 1024 {
            return None;
        }
    }
    let text = String::from_utf8_lossy(&head).into_owned();
    let mut lines = text.split("\r\n");
    let path = lines.next()?.split(' ').nth(1)?.to_owned();
    let headers = lines
        .filter_map(|line| line.split_once(": "))
        .map(|(n, v)| (n.to_owned(), v.to_owned()))
        .collect();
    Some(Request { path, headers })
}

async fn answer<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, reply: Reply) {
    let _ = stream.write_all(reply.head.as_bytes()).await;
    match reply.body {
        Body::Bytes(bytes) => {
            let _ = stream.write_all(&bytes).await;
        }
        Body::Stream {
            prefix,
            fill,
            len,
            written,
        } => {
            let mut sent = 0;
            let mut next = prefix;
            let filler = vec![fill; 16 * 1024];
            while sent < len {
                if next.is_empty() {
                    next = filler[..filler.len().min(len - sent)].to_vec();
                }
                if stream.write_all(&next).await.is_err() {
                    break;
                }
                sent += next.len();
                written.store(sent, Ordering::SeqCst);
                next = Vec::new();
            }
        }
        Body::Hang => tokio::time::sleep(std::time::Duration::from_secs(30)).await,
    }
    let _ = stream.shutdown().await;
}

/// Serves HTTP on an ephemeral loopback port; `tls` makes it HTTPS.
pub async fn serve(
    tls: Option<TlsAcceptor>,
    handler: impl Fn(&Request) -> Reply + Send + Sync + 'static,
) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = Server {
        addr: listener.local_addr().unwrap(),
        requests: Arc::default(),
        connections: Arc::default(),
    };
    let (requests, connections) = (server.requests.clone(), server.connections.clone());
    let handler: Responder = Arc::new(handler);
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            connections.fetch_add(1, Ordering::SeqCst);
            let (requests, handler, tls) = (requests.clone(), handler.clone(), tls.clone());
            tokio::spawn(async move {
                let Some(acceptor) = tls else {
                    return handle(tcp, requests, handler).await;
                };
                if let Ok(stream) = acceptor.accept(tcp).await {
                    handle(stream, requests, handler).await;
                }
            });
        }
    });
    server
}

type Responder = Arc<dyn Fn(&Request) -> Reply + Send + Sync>;

async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    requests: Arc<Mutex<Vec<Request>>>,
    handler: Responder,
) {
    if let Some(request) = read_request(&mut stream).await {
        requests.lock().unwrap().push(request.clone());
        answer(stream, handler(&request)).await;
    }
}

/// A self-signed certificate valid between the two years (January 1st).
pub fn acceptor(from_year: i32, to_year: i32) -> TlsAcceptor {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
    params.not_before = rcgen::date_time_ymd(from_year, 1, 1);
    params.not_after = rcgen::date_time_ymd(to_year, 1, 1);
    let cert = params.self_signed(&key).unwrap();
    let key_der = PrivatePkcs8KeyDer::from(key.serialize_der());
    let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert.der().clone()], key_der.into())
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

/// A UDP DNS server that answers every query with one record. Returns its port.
pub async fn dns_server(record_type: u16, rdata: Vec<u8>) -> u16 {
    dns_records(vec![(record_type, rdata)]).await
}

/// A UDP DNS server that answers every query with all the given `(type, rdata)` records.
pub async fn dns_records(records: Vec<(u16, Vec<u8>)>) -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    tokio::spawn(async move {
        let mut buf = [0u8; 512];
        while let Ok((len, from)) = socket.recv_from(&mut buf).await {
            let query = &buf[..len];
            let end = 12 + query[12..].iter().position(|b| *b == 0).unwrap() + 5;
            let count = u8::try_from(records.len()).unwrap();
            let mut reply = vec![query[0], query[1], 0x81, 0x80, 0, 1, 0, count, 0, 0, 0, 0];
            reply.extend_from_slice(&query[12..end]);
            for (record_type, rdata) in &records {
                reply.extend_from_slice(&[0xC0, 0x0C]);
                reply.extend_from_slice(&record_type.to_be_bytes());
                reply.extend_from_slice(&[0, 1, 0, 0, 0, 60]);
                reply.extend_from_slice(&u16::try_from(rdata.len()).unwrap().to_be_bytes());
                reply.extend_from_slice(rdata);
            }
            let _ = socket.send_to(&reply, from).await;
        }
    });
    port
}

/// A UDP socket that reads queries and never answers. Returns its port and keeps it alive.
pub async fn silent_dns_server() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    tokio::spawn(async move {
        let mut buf = [0u8; 512];
        while socket.recv_from(&mut buf).await.is_ok() {}
    });
    port
}

/// Wire form of a domain name.
pub fn wire_name(name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for label in name.trim_end_matches('.').split('.') {
        out.push(u8::try_from(label.len()).unwrap());
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    out
}
