//! The hub's side of the one interface of the proxy (Section 12.3, SF-NEW-routeproxyexit).
//!
//! The proxy runs as a process apart from the hub. It reaches the hub only through this Unix
//! socket, with the requests of `anima_route_proxy::wire`: the table of the names that serve,
//! a connection to an application by its identifier, and its counts. The hub maps the
//! identifier to the device and the port of its own record ([`super::target_of`]); no request
//! can name a device, an address or a port, so the proxy can never reach port 22 of a machine.
//! The socket is served only when `ANIMA_ROUTES_SOCKET` is set: off by default.
//!
//! Each link has a limit of public streams at one time, so the public traffic leaves room for
//! the sessions of the operator on the same tunnel (SF-NEW-routelinkshare). I/O edge; it writes
//! no body and no header to a log (LOG-007).

use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex, PoisonError},
};

use anima_route_proxy::wire::{self, Answer, Named, Request, Table};
use async_trait::async_trait;
use sqlx::PgPool;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt as _, BufReader, copy_bidirectional},
    net::UnixListener,
};
use zeroize::Zeroizing;

use super::{Traffic, count, served, target_of};
use crate::{
    clock::Clock,
    fleet::{
        credentials::Vault,
        targets::{self, AddressPolicy},
    },
    ids::{ApplicationId, DeviceId},
};

/// A stream to the port of an application.
pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Stream for T {}

/// How the hub reaches a fixed target: the hub's one connect function, or a double in tests.
#[async_trait]
pub trait Connector: Send + Sync {
    async fn connect(&self, address: SocketAddr) -> io::Result<Box<dyn Stream>>;
}

/// The hub's one connect function (task HM-T12), through the link for a link address.
pub struct Network(pub AddressPolicy);

#[async_trait]
impl Connector for Network {
    async fn connect(&self, address: SocketAddr) -> io::Result<Box<dyn Stream>> {
        let reached = targets::connect(&self.0, address).await?;
        Ok(Box::new(reached))
    }
}

/// The public streams at one time on each link (SF-NEW-routelinkshare).
#[derive(Clone)]
pub struct LinkStreams {
    limit: usize,
    open: Arc<Mutex<HashMap<DeviceId, usize>>>,
}

/// One stream that counts against its link until it is dropped.
pub struct StreamPermit {
    device: DeviceId,
    open: Arc<Mutex<HashMap<DeviceId, usize>>>,
}

impl LinkStreams {
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            open: Arc::default(),
        }
    }

    /// A permit for one more stream to `device`; `None` at the limit.
    #[must_use]
    pub fn take(&self, device: DeviceId) -> Option<StreamPermit> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        let streams = open.entry(device).or_default();
        if *streams >= self.limit {
            return None;
        }
        *streams += 1;
        Some(StreamPermit {
            device,
            open: Arc::clone(&self.open),
        })
    }
}

impl Drop for StreamPermit {
    fn drop(&mut self) {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(streams) = open.get_mut(&self.device) {
            *streams = streams.saturating_sub(1);
        }
    }
}

/// What the interface needs.
pub struct Door {
    pub db: PgPool,
    pub vault: Vault,
    pub clock: Arc<dyn Clock>,
    pub connector: Arc<dyn Connector>,
    pub streams: LinkStreams,
}

/// Serves the interface on `listener` until the task is stopped.
pub async fn serve(listener: UnixListener, door: Arc<Door>) {
    loop {
        let Ok((socket, _)) = listener.accept().await else {
            continue;
        };
        let door = Arc::clone(&door);
        tokio::spawn(async move {
            if let Err(error) = answer(&door, socket).await {
                tracing::warn!(error = %error, "a request of the route proxy failed");
            }
        });
    }
}

/// Why one request was not answered. Holds no key and no byte of a stream.
#[derive(Debug, thiserror::Error)]
pub enum DoorError {
    #[error(transparent)]
    Wire(#[from] wire::WireError),
    #[error("the routes cannot be read")]
    Routes(#[from] super::Error),
    #[error("the stream ended with an error")]
    Stream(#[from] io::Error),
}

/// Answers the one request on `socket`.
///
/// # Errors
/// [`DoorError`] for a request that is not one of the interface, or a failure of the hub.
pub async fn answer<S: AsyncRead + AsyncWrite + Unpin + Send>(
    door: &Door,
    socket: S,
) -> Result<(), DoorError> {
    let mut socket = BufReader::new(socket);
    let Some(line) = wire::read_line(&mut socket, wire::REQUEST_MAX).await? else {
        return Ok(());
    };
    let now = door.clock.now();
    match wire::parse_request(&line)? {
        Request::Table => {
            let table = Table {
                names: served(&door.db, &door.vault, now)
                    .await?
                    .into_iter()
                    .map(|route| Named {
                        name: route.name.to_string(),
                        application: route.application.into(),
                        chain_pem: route.chain_pem,
                        key_pem: Zeroizing::new(
                            String::from_utf8_lossy(route.key_pem.expose()).into_owned(),
                        ),
                    })
                    .collect(),
                challenges: Vec::new(),
            };
            wire::write_json(&mut socket, &table).await?;
        }
        Request::Connect { application } => {
            connect(door, &mut socket, ApplicationId::from(application)).await?;
        }
        Request::Count { name, counts } => {
            let traffic = Traffic {
                requests: counts.requests,
                bytes_in: counts.bytes_in,
                bytes_out: counts.bytes_out,
                refused: counts.refused,
            };
            count(&door.db, &name, traffic, now).await?;
            say(&mut socket, Answer::Ok).await?;
        }
    }
    Ok(())
}

/// Connects the socket to the fixed target of `application`, or answers why not.
async fn connect<S: AsyncRead + AsyncWrite + Unpin + Send>(
    door: &Door,
    socket: &mut BufReader<S>,
    application: ApplicationId,
) -> Result<(), DoorError> {
    let (device, address) = match target_of(&door.db, application, door.clock.now()).await {
        Ok(found) => found,
        Err(super::Error::NotFound) => return say(socket, Answer::Refused).await,
        Err(error) => return Err(error.into()),
    };
    let Some(_permit) = door.streams.take(device) else {
        return say(socket, Answer::Refused).await;
    };
    let Ok(mut stream) = door.connector.connect(address).await else {
        return say(socket, Answer::Unavailable).await;
    };
    say(socket, Answer::Ok).await?;
    copy_bidirectional(socket, &mut stream).await?;
    Ok(())
}

async fn say<S: AsyncWrite + Unpin>(socket: &mut S, answer: Answer) -> Result<(), DoorError> {
    socket.write_all(answer.word().as_bytes()).await?;
    socket.write_all(b"\n").await?;
    socket.flush().await?;
    Ok(())
}
