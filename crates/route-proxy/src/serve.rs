//! The listeners of the proxy: TLS for the names that serve, and plain HTTP for the ACME
//! challenge and the redirect (Section 12.2 and 12.3). Each listener caps its connections in all
//! and for each client before the TLS handshake, and each step has a time limit
//! (SF-NEW-revroutelimits). Each connection is counted and limited for its name. The log has only the name, the bytes, the status and the time: never a path, a
//! query, a header or a cookie (LOG-007, SF-NEW-routeheaders). I/O edge.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex, PoisonError, RwLock},
    time::Instant,
};

use rustls::ServerConfig;
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

use crate::{
    door_header,
    head::{self, HEAD_MAX, Status},
    hub::Hub,
    limits::{Gate, Limiter, Pass, Timeouts},
    plain,
    relay::{Idle, relay},
    settings::Settings,
    tls,
    wire::Counts,
};

/// The answer to a connection that a limit of its name refused.
const OVER_LIMIT: &[u8] =
    b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
/// The answer when the hub cannot reach the application.
const NO_APPLICATION: &[u8] =
    b"HTTP/1.1 502 Bad Gateway\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";

/// The state that each connection shares.
pub struct Proxy<H: Hub> {
    hub: H,
    /// Whether each connection comes from the door with a PROXY header.
    door: bool,
    tls: RwLock<Arc<ServerConfig>>,
    names: RwLock<HashMap<String, Uuid>>,
    challenges: RwLock<HashMap<String, String>>,
    limiter: Mutex<Limiter>,
    timeouts: Timeouts,
    tls_gate: Arc<Mutex<Gate>>,
    plain_gate: Arc<Mutex<Gate>>,
    counts: Mutex<HashMap<String, Counts>>,
    started: Instant,
}

impl<H: Hub> Proxy<H> {
    /// A proxy with no name until the first [`Proxy::refresh`].
    ///
    /// # Errors
    /// When the TLS library has no safe default.
    pub fn new(hub: H, settings: &Settings) -> Result<Self, rustls::Error> {
        let (config, _) = tls::server_config(&crate::wire::Table::default())?;
        Ok(Self {
            hub,
            door: settings.door,
            tls: RwLock::new(config),
            names: RwLock::default(),
            challenges: RwLock::default(),
            limiter: Mutex::new(Limiter::new(settings.limits)),
            timeouts: settings.timeouts,
            tls_gate: Arc::new(Mutex::new(Gate::new(settings.tls_caps))),
            plain_gate: Arc::new(Mutex::new(Gate::new(settings.plain_caps))),
            counts: Mutex::default(),
            started: Instant::now(),
        })
    }

    /// Reads the table of the hub again; returns the names that serve and those left out.
    ///
    /// # Errors
    /// When the hub does not answer, or the TLS library refuses.
    pub async fn refresh(&self) -> Result<(usize, Vec<String>), Box<dyn std::error::Error>> {
        let table = self.hub.table().await?;
        let (config, left_out) = tls::server_config(&table)?;
        let names: HashMap<String, Uuid> = table
            .names
            .iter()
            .filter(|named| !left_out.contains(&named.name))
            .map(|named| (named.name.clone(), named.application))
            .collect();
        let challenges = table
            .challenges
            .into_iter()
            .map(|challenge| (challenge.token, challenge.key_authorization))
            .collect();
        let serving = names.len();
        *self.tls.write().unwrap_or_else(PoisonError::into_inner) = config;
        *self.names.write().unwrap_or_else(PoisonError::into_inner) = names;
        *self
            .challenges
            .write()
            .unwrap_or_else(PoisonError::into_inner) = challenges;
        Ok((serving, left_out))
    }

    /// Gives the hub what was counted since the last report.
    pub async fn report(&self) {
        let counts =
            std::mem::take(&mut *self.counts.lock().unwrap_or_else(PoisonError::into_inner));
        for (name, counts) in counts {
            if let Err(error) = self.hub.count(&name, counts).await {
                tracing::warn!(error = %error, "the hub did not take the counts");
            }
        }
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn add(&self, name: &str, change: impl FnOnce(&mut Counts)) {
        let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        change(counts.entry(name.to_owned()).or_default());
    }
}

/// Serves TLS on `listener` until the task is stopped. A connection over the cap is closed
/// before any byte is read.
pub async fn serve_tls<H: Hub>(proxy: Arc<Proxy<H>>, listener: TcpListener) {
    loop {
        let Ok((stream, peer)) = listener.accept().await else {
            continue;
        };
        let Some(pass) = Pass::enter(&proxy.tls_gate) else {
            continue;
        };
        let proxy = Arc::clone(&proxy);
        tokio::spawn(async move { one_tls(&proxy, stream, peer.ip(), pass).await });
    }
}

/// Serves plain HTTP on `listener` until the task is stopped.
pub async fn serve_plain<H: Hub>(proxy: Arc<Proxy<H>>, listener: TcpListener) {
    loop {
        let Ok((stream, peer)) = listener.accept().await else {
            continue;
        };
        let Some(pass) = Pass::enter(&proxy.plain_gate) else {
            continue;
        };
        let proxy = Arc::clone(&proxy);
        tokio::spawn(async move { one_plain(&proxy, stream, peer.ip(), pass).await });
    }
}

/// One plain connection: the challenge or the redirect, each step within its time.
async fn one_plain<H: Hub>(proxy: &Proxy<H>, mut stream: TcpStream, peer: IpAddr, mut pass: Pass) {
    let times = proxy.timeouts;
    let Some(client) = client(proxy, &mut stream, peer).await else {
        return;
    };
    if !pass.admit(client) {
        return;
    }
    let Ok(Ok((head, _))) = timeout(times.head, read_head(&mut stream)).await else {
        return;
    };
    let Some((host, target)) = plain::host_and_target(&head) else {
        return;
    };
    let names: Vec<String> = (proxy.names.read().unwrap_or_else(PoisonError::into_inner))
        .keys()
        .cloned()
        .collect();
    let challenges = proxy
        .challenges
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let answer = plain::answer(&host, &target, &names, &challenges);
    timeout(times.idle, stream.write_all(&answer)).await.ok();
}

/// The client of a connection: from the PROXY header of the door within the handshake time,
/// or the peer; `None` closes the connection.
async fn client<H: Hub>(proxy: &Proxy<H>, stream: &mut TcpStream, peer: IpAddr) -> Option<IpAddr> {
    if !proxy.door {
        return Some(peer);
    }
    timeout(proxy.timeouts.handshake, client_of_door(stream))
        .await
        .ok()
        .flatten()
}

/// One TLS connection: the name of its SNI, its limits, its request and its answer.
async fn one_tls<H: Hub>(proxy: &Proxy<H>, mut stream: TcpStream, peer: IpAddr, mut pass: Pass) {
    let times = proxy.timeouts;
    let started = tokio::time::Instant::now();
    let Some(client) = client(proxy, &mut stream, peer).await else {
        return;
    };
    if !pass.admit(client) {
        return;
    }
    let config = Arc::clone(&proxy.tls.read().unwrap_or_else(PoisonError::into_inner));
    let left = times.handshake.saturating_sub(started.elapsed());
    let Ok(Ok(tls)) = timeout(left, TlsAcceptor::from(config).accept(stream)).await else {
        return;
    };
    let Some(name) = tls.get_ref().1.server_name().map(str::to_owned) else {
        return;
    };
    let mut tls = Idle::new(tls, times.idle);
    let Some(application) = proxy
        .names
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&name)
        .copied()
    else {
        return;
    };
    let admitted = proxy
        .limiter
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .admit(&name, proxy.now_ms());
    if admitted.is_err() {
        proxy.add(&name, |counts| counts.refused += 1);
        tls.write_all(OVER_LIMIT).await.ok();
        return;
    }
    let status = request(proxy, &mut tls, (&name, application), client).await;
    proxy
        .limiter
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .release(&name);
    tracing::info!(name = %name, status = ?status, "a request of a public name");
}

/// Reads, checks and relays one request; returns the status sent to the client.
async fn request<H: Hub, S: AsyncRead + AsyncWrite + Unpin + Send>(
    proxy: &Proxy<H>,
    tls: &mut S,
    (name, application): (&str, Uuid),
    client: IpAddr,
) -> Option<u16> {
    let (rewritten, early) = match read_request(proxy.timeouts.head, tls, name, client).await {
        Ok(read) => read,
        Err(status) => {
            tls.write_all(&status.answer()).await.ok();
            return Some(status.code());
        }
    };
    let Ok(app) = proxy.hub.connect(application).await else {
        tls.write_all(NO_APPLICATION).await.ok();
        return Some(502);
    };
    let mut app = Idle::new(app, proxy.timeouts.idle);
    let relayed = relay(tls, &early, &mut app, &rewritten).await.ok()?;
    proxy.add(name, |counts| {
        counts.requests += 1;
        counts.bytes_in += relayed.bytes_in;
        counts.bytes_out += relayed.bytes_out;
    });
    let total = relayed.bytes_in + relayed.bytes_out;
    proxy
        .limiter
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .add_bytes(name, total, proxy.now_ms());
    tracing::debug!(name = %name, bytes_in = relayed.bytes_in, bytes_out = relayed.bytes_out, "relayed");
    relayed.status
}

/// The head of one request within `wait`, written again for the application of `name`, and
/// the bytes after it.
async fn read_request<S: AsyncRead + Unpin>(
    wait: std::time::Duration,
    tls: &mut S,
    name: &str,
    client: IpAddr,
) -> Result<(head::Rewritten, Vec<u8>), Status> {
    let (head, early) = match timeout(wait, read_head(tls)).await {
        Ok(Ok(read)) => read,
        Ok(Err(NoHead)) => return Err(Status::HeadTooLarge),
        Err(_) => return Err(Status::RequestTimeout),
    };
    Ok((head::rewrite(&head, name, client)?, early))
}

/// Why a head was not read.
#[derive(Debug)]
struct NoHead;

/// Reads up to the end of a request head: the head and the bytes after it.
async fn read_head<S: AsyncRead + Unpin>(stream: &mut S) -> Result<(Vec<u8>, Vec<u8>), NoHead> {
    let mut buffer = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream.read(&mut chunk).await.map_err(|_| NoHead)?;
        if read == 0 {
            return Err(NoHead);
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(end) = head::end_of_head(&buffer) {
            let early = buffer.split_off(end);
            return Ok((buffer, early));
        }
        if buffer.len() > HEAD_MAX {
            return Err(NoHead);
        }
    }
}

/// The client address of the PROXY header of the door; `None` closes the connection.
async fn client_of_door(stream: &mut TcpStream) -> Option<IpAddr> {
    let mut line = Vec::with_capacity(door_header::HEADER_MAX);
    while line.len() < door_header::HEADER_MAX {
        let byte = stream.read_u8().await.ok()?;
        if byte == b'\n' {
            let text = std::str::from_utf8(&line).ok()?;
            return door_header::client(text.strip_suffix('\r')?);
        }
        line.push(byte);
    }
    None
}
