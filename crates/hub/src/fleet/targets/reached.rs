//! The hub's one connect function: a TCP connection to a checked address, by the network or,
//! for a link address, through the link (HMD-048).
//!
//! No packet goes to a link address. The computer that has it keeps streams open to the link
//! of the hub, and the link gives the hub a connection to one local port of that computer
//! through its dial socket (`link/dial`): the hub writes `dial <link address> <port>`, the
//! link answers `ok` and the bytes of the port follow, or it answers one word that says why
//! not. The computer decides which of its ports the hub may reach (HMD-049).
//!
//! The dial socket must be served by the hub's own user. What goes through it has its own
//! protection: an SSH connection pins the host key before a credential leaves the hub
//! (SEC-007). I/O edge; nothing here logs.

use std::{
    io,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    path::Path,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, ReadBuf},
    net::TcpStream,
};

#[cfg(windows)]
use anima_windows::socket::Stream as UnixStream;
#[cfg(unix)]
use tokio::net::UnixStream;

use super::AddressPolicy;
use crate::config::LINK_DEVICES;

/// The longest the link may take to answer a request: its own wait for the computer is 10 s.
const LINK_ANSWER: Duration = Duration::from_secs(15);
/// The longest answer word of the link, with its line end.
const WORD_MAX: usize = 32;

/// A connection to a TCP port of a device.
#[derive(Debug)]
pub enum Reached {
    /// By the network.
    Tcp(TcpStream),
    /// Through the link, to a local port of a computer on it.
    Link(UnixStream),
}

impl Reached {
    /// The address at the other end of a connection by the network; `None` through the link,
    /// where the other end is a local port of the computer.
    #[must_use]
    pub fn network_peer(&self) -> Option<IpAddr> {
        match self {
            Self::Tcp(stream) => stream.peer_addr().ok().map(|peer| peer.ip()),
            Self::Link(_) => None,
        }
    }
}

/// Connects to `address`, which the caller has checked against `policy`: through the link
/// when it is a link address and the hub has a link ([`AddressPolicy::reaching_link`]), by the
/// network otherwise.
///
/// # Errors
/// As a TCP connect by the network. Through the link: `PermissionDenied` when the computer
/// has not turned the port on, `ConnectionRefused` when nothing listens on it, `NotConnected`
/// when the computer is not on the link, `TimedOut` when it did not answer, and
/// `InvalidData` for any other answer of the link.
pub async fn connect(policy: &AddressPolicy, address: SocketAddr) -> io::Result<Reached> {
    if let (Some(socket), IpAddr::V6(device)) = (policy.link_dial(), address.ip())
        && LINK_DEVICES.contains(&device)
    {
        let dialled = dial(socket, device, address.port());
        return tokio::time::timeout(LINK_ANSWER, dialled)
            .await
            .map_err(|_| io::ErrorKind::TimedOut)?
            .map(Reached::Link);
    }
    TcpStream::connect(address).await.map(Reached::Tcp)
}

/// Whether `ip` is a link address: a computer that the hub reaches with TCP only, through
/// the link. A check that needs ICMP or UDP cannot run on it (HMD-048).
#[must_use]
pub fn on_link(ip: IpAddr) -> bool {
    matches!(ip, IpAddr::V6(device) if LINK_DEVICES.contains(&device))
}

/// One request on the dial socket of the link.
async fn dial(socket: &Path, device: Ipv6Addr, port: u16) -> io::Result<UnixStream> {
    let mut stream = UnixStream::connect(socket).await?;
    #[cfg(unix)]
    if stream.peer_cred()?.uid() != rustix::process::geteuid().as_raw() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    stream
        .write_all(format!("dial {device} {port}\n").as_bytes())
        .await?;
    // One byte at a time: the bytes after the line are the device's.
    let mut word = Vec::with_capacity(WORD_MAX);
    loop {
        match stream.read_u8().await? {
            b'\n' => break,
            _ if word.len() == WORD_MAX => return Err(io::ErrorKind::InvalidData.into()),
            byte => word.push(byte),
        }
    }
    let refusal = match word.as_slice() {
        b"ok" => return Ok(stream),
        b"refused" => io::ErrorKind::PermissionDenied,
        b"closed" => io::ErrorKind::ConnectionRefused,
        b"not_connected" => io::ErrorKind::NotConnected,
        b"timeout" => io::ErrorKind::TimedOut,
        _ => io::ErrorKind::InvalidData,
    };
    Err(refusal.into())
}

impl AsyncRead for Reached {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(stream) => Pin::new(stream).poll_read(context, buffer),
            Self::Link(stream) => Pin::new(stream).poll_read(context, buffer),
        }
    }
}

impl AsyncWrite for Reached {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Tcp(stream) => Pin::new(stream).poll_write(context, bytes),
            Self::Link(stream) => Pin::new(stream).poll_write(context, bytes),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(stream) => Pin::new(stream).poll_flush(context),
            Self::Link(stream) => Pin::new(stream).poll_flush(context),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(stream) => Pin::new(stream).poll_shutdown(context),
            Self::Link(stream) => Pin::new(stream).poll_shutdown(context),
        }
    }
}
