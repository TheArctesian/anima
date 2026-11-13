//! The UDP socket of one SNMP attempt: bound for the attempt, it sends to the one target and
//! reads only datagrams from that address and port.

use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    time::{Duration, Instant},
};

use tokio::net::UdpSocket;
use zeroize::Zeroizing;

use super::message::DATAGRAM_MAX;
use crate::fleet::checks::probe::Failure;

pub(in crate::fleet::checks) struct Link {
    socket: UdpSocket,
    peer: SocketAddr,
    /// Wiped on drop: a v2c reply repeats the community.
    buffer: Zeroizing<Vec<u8>>,
}

impl Link {
    /// A socket on an ephemeral port of the target's address family.
    ///
    /// # Errors
    /// The socket cannot be bound.
    pub(in crate::fleet::checks) async fn open(peer: SocketAddr) -> Result<Self, Failure> {
        let local = if peer.is_ipv4() {
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))
        } else {
            SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0))
        };
        Ok(Self {
            socket: UdpSocket::bind(local).await?,
            peer,
            buffer: Zeroizing::new(vec![0; DATAGRAM_MAX]),
        })
    }

    /// Sends `request` and reads replies until `read` takes one, with the round-trip time.
    /// A datagram from another address or port is dropped unread; `read` gives `None` for a
    /// reply to another request. The caller's time limit ends the wait.
    ///
    /// # Errors
    /// The send or receive fails, or `read` gives a failure.
    pub(in crate::fleet::checks) async fn exchange<T>(
        &mut self,
        request: &[u8],
        mut read: impl FnMut(&[u8]) -> Option<Result<T, Failure>>,
    ) -> Result<(T, Duration), Failure> {
        if request.len() > DATAGRAM_MAX {
            return Err(Failure::BadConfig);
        }
        let started = Instant::now();
        self.socket.send_to(request, self.peer).await?;
        loop {
            let (length, from) = self.socket.recv_from(&mut self.buffer).await?;
            if from.ip() != self.peer.ip() || from.port() != self.peer.port() {
                continue;
            }
            if let Some(answer) = read(&self.buffer[..length]) {
                return answer.map(|value| (value, started.elapsed()));
            }
        }
    }
}
