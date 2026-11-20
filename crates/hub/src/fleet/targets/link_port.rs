//! A port on the loopback address of the hub that carries to one port of a computer on the
//! link (HMD-048): for a program beside the hub that connects by TCP only, as guacd does for
//! VNC and RDP. guacd cannot reach a link address, because no packet goes to one; the hub
//! gives it this port in its place and carries each connection through the link with
//! [`connect`], so the computer still decides which of its ports are open (HMD-049).
//!
//! The port lives as long as its [`LinkPort`]. While it lives, each program on the server of
//! the hub can connect to it; a hub with a link is the only user of its server (`anima-local-hub`),
//! and what the port leads to has its own sign-in (the VNC password, the RDP sign-in). I/O
//! edge; nothing here logs.

use std::{
    io,
    net::{Ipv4Addr, SocketAddr},
};

use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};

use super::{AddressPolicy, connect, on_link};

/// One loopback port of the hub and the task that carries its connections.
#[derive(Debug)]
pub struct LinkPort {
    local: SocketAddr,
    carrying: JoinHandle<()>,
}

impl LinkPort {
    /// Opens a loopback port that carries to `target`, when `target` is a link address and
    /// the hub has a link; `None` for each other address, which a program reaches by itself.
    ///
    /// # Errors
    /// When no loopback port can be taken.
    pub async fn open(policy: &AddressPolicy, target: SocketAddr) -> io::Result<Option<Self>> {
        if !(on_link(target.ip()) && policy.link_dial().is_some()) {
            return Ok(None);
        }
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let local = listener.local_addr()?;
        let carrying = tokio::spawn(carry_all(listener, policy.clone(), target));
        Ok(Some(Self { local, carrying }))
    }

    /// The address that the program beside the hub connects to.
    #[must_use]
    pub const fn local(&self) -> SocketAddr {
        self.local
    }
}

impl Drop for LinkPort {
    /// Closes the port and each connection that it carries.
    fn drop(&mut self) {
        self.carrying.abort();
    }
}

/// Takes each connection from this server and carries it to `target`. The connections are
/// tasks of this one, so they end with it.
async fn carry_all(listener: TcpListener, policy: AddressPolicy, target: SocketAddr) {
    let mut carried = JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((local, peer)) = accepted else { return };
                if peer.ip().is_loopback() {
                    carried.spawn(carry(local, policy.clone(), target));
                }
            }
            Some(_) = carried.join_next(), if !carried.is_empty() => {}
        }
    }
}

/// One connection: its bytes go to `target` through the link and back, until one side ends.
/// When the computer does not give the port, the connection is closed with no byte.
async fn carry(mut local: TcpStream, policy: AddressPolicy, target: SocketAddr) {
    if let Ok(mut far) = connect(&policy, target).await {
        let _ended = copy_bidirectional(&mut local, &mut far).await;
    }
}
