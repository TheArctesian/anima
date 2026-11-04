//! Everything the tunnel does before the WebSocket upgrade, after the ticket is redeemed:
//! load the device, resolve and check its host (R-032), check its host key (R-034), build the
//! `connect` values and run the guacd handshake (K4 Section 3).

use std::{future::Future, net::SocketAddr, pin::Pin, sync::Arc};

use guac_protocol::{ConnectParams, DecodeError};
use tokio::{net::lookup_host, time::timeout};

use super::{
    arguments::{BadConnectValue, connect_values, time_zone},
    guacd::{self, Connected},
    route::Route,
};
use crate::{
    credentials,
    devices::{self, Device},
    sessions::{self, PendingTicket, PublicKey, start::is_refused_target},
};

/// The key a device presented and the address it was read from: the address the reader
/// resolved the host to and checked against the target rule (WP-10 T3 `KeyRead`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HostKeyRead {
    pub(super) key: PublicKey,
    pub(super) address: SocketAddr,
}

/// Why the reader found no key, in the tunnel's terms (from WP-10 T3 `ProbeError`: its
/// `Unresolved` and `RefusedTarget` map to the same names, everything else to `Unreadable`).
/// Holds no detail: the reader logs its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "built by the WP-10 T3 probe once tunnel_router wires it in"
    )
)]
pub(super) enum HostKeyUnread {
    Unresolved,
    RefusedTarget,
    Unreadable,
}

/// Reads the SSH host key the device presents at `host` and `port`, without signing in
/// (R-034, the `testConnection` probe). The reader resolves the host and applies the target
/// rule, so the tunnel resolves nothing itself and sends guacd the address the key came
/// from. The tunnel bounds the wait.
pub(super) type ReadHostKey = Arc<
    dyn Fn(String, u16) -> Pin<Box<dyn Future<Output = Result<HostKeyRead, HostKeyUnread>> + Send>>
        + Send
        + Sync,
>;

/// What the hub's own read of the host key found (R-034).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HostKeyCheck {
    /// The key read is the accepted one, or the hub has no reader and leaves it to guacd's pin.
    Accepted,
    /// The device presents another key: the session ends `HOST_KEY_MISMATCH`.
    Changed,
}

/// What the tunnel opens after the upgrade.
pub(super) enum Opening {
    /// guacd is past `ready`: relay.
    Gateway(Connected),
    /// The host key changed: tell the client, close 4001.
    HostKeyChanged,
}

/// Why the tunnel did not open. The messages are for the process log; none holds a value,
/// an address, guacd's text or the token (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(super) enum NotOpened {
    #[error("the device no longer exists")]
    DeviceGone,
    #[error("SSH is turned off for the device")]
    SshOff,
    #[error("no host key is accepted for the device")]
    NoHostKey,
    #[error("the device host does not resolve")]
    Unresolved,
    #[error("the device host resolves to a refused address")]
    RefusedTarget,
    #[error("the device's host key could not be read")]
    HostKeyUnread,
    #[error(transparent)]
    Values(#[from] BadConnectValue),
    #[error("guacd cannot be reached")]
    Unreachable,
    #[error("the guacd handshake did not finish in time")]
    HandshakeTimeout,
    #[error("guacd closed the connection during the handshake")]
    GatewayClosed,
    #[error("guacd refused the handshake (status {status:?})")]
    Refused { status: Option<u16> },
    #[error("guacd broke the handshake")]
    Broken,
    #[error("guacd sent bytes that do not decode")]
    Garbled(#[source] DecodeError),
    #[error("device store error")]
    Devices(#[source] devices::Error),
    #[error("session store error")]
    Sessions(#[source] sessions::Error),
    #[error("credential store error")]
    Credentials(#[source] credentials::Error),
}

impl NotOpened {
    /// The event message: what the operator can do about it, with no detail from guacd.
    pub(super) const fn message(&self) -> &'static str {
        match self {
            Self::DeviceGone => "The device was deleted before the SSH session opened.",
            Self::SshOff => "SSH was turned off for the device before the session opened.",
            Self::NoHostKey => {
                "The device has no accepted host key. Test the connection and accept its key."
            }
            Self::Unresolved => "The device's host name does not resolve.",
            Self::RefusedTarget => {
                "The device's host resolves to a loopback, link-local, multicast or unspecified \
                 address, which the gateway must not reach."
            }
            Self::HostKeyUnread => "The hub could not read the device's SSH host key.",
            Self::Values(_) => {
                "The device's settings or stored credential cannot be sent to the gateway."
            }
            Self::Unreachable => "The hub could not reach the gateway.",
            Self::HandshakeTimeout
            | Self::GatewayClosed
            | Self::Refused { .. }
            | Self::Broken
            | Self::Garbled(_) => "The gateway did not open the SSH session.",
            Self::Devices(_) | Self::Sessions(_) | Self::Credentials(_) => {
                "The hub could not read the session's settings from its database."
            }
        }
    }
}

/// Opens the gateway side of the ticket's session, or learns that the host key changed.
///
/// # Errors
/// [`NotOpened`] for each step that fails; the caller ends the session `GATEWAY_ERROR`.
pub(super) async fn open(route: &Route, ticket: &PendingTicket) -> Result<Opening, NotOpened> {
    let device = devices::get(&route.db, ticket.device)
        .await
        .map_err(|error| match error {
            devices::Error::NotFound(_) => NotOpened::DeviceGone,
            other => NotOpened::Devices(other),
        })?;
    if !device.ssh.enabled {
        return Err(NotOpened::SshOff);
    }
    let accepted = sessions::host_key(&route.db, device.id)
        .await
        .map_err(NotOpened::Sessions)?
        .ok_or(NotOpened::NoHostKey)?;
    let (target, check) = check_host_key(route, &device, &accepted.public_key).await?;
    if check == HostKeyCheck::Changed {
        return Ok(Opening::HostKeyChanged);
    }
    let secret = credentials::load_ssh(&route.db, &route.vault, device.id)
        .await
        .map_err(NotOpened::Credentials)?;
    let values = connect_values(&device, target.ip(), secret.as_ref(), &accepted)?;
    drop(secret);
    let params = ConnectParams {
        display: ticket.display,
        timezone: time_zone(ticket.timezone.as_deref()),
        name: ticket.operator.name.clone(),
        values,
    };
    guacd::connect(&route.guacd, params, route.timing)
        .await
        .map(Opening::Gateway)
}

/// The first address the device's host resolves to, unless the gateway must not reach it
/// (`start`'s rule: loopback, link-local, multicast, broadcast, unspecified, "this network",
/// or an IPv6 address embedding one of those). An IPv4-mapped address is given as IPv4.
async fn target(host: &str, port: u16) -> Result<SocketAddr, NotOpened> {
    let first = lookup_host((host, port))
        .await
        .ok()
        .and_then(|mut addresses| addresses.next())
        .ok_or(NotOpened::Unresolved)?;
    let ip = first.ip().to_canonical();
    if is_refused_target(ip) {
        return Err(NotOpened::RefusedTarget);
    }
    Ok(SocketAddr::new(ip, port))
}

/// The hub's own host-key check before the guacd handshake (R-034), and the address guacd
/// is sent to: the one the key was read from. Without a reader the host is resolved and
/// checked here, and the key is left to guacd's pin.
async fn check_host_key(
    route: &Route,
    device: &Device,
    accepted: &PublicKey,
) -> Result<(SocketAddr, HostKeyCheck), NotOpened> {
    let (host, port) = (device.host.clone(), device.ssh.port);
    let wait = route.timing.connect;
    let Some(read) = &route.read_host_key else {
        let target = timeout(wait, target(&host, port))
            .await
            .map_err(|_| NotOpened::Unresolved)??;
        return Ok((target, HostKeyCheck::Accepted));
    };
    let read = timeout(wait, read(host, port))
        .await
        .map_err(|_| NotOpened::HostKeyUnread)?
        .map_err(|unread| match unread {
            HostKeyUnread::Unresolved => NotOpened::Unresolved,
            HostKeyUnread::RefusedTarget => NotOpened::RefusedTarget,
            HostKeyUnread::Unreadable => NotOpened::HostKeyUnread,
        })?;
    let check = if read.key == *accepted {
        HostKeyCheck::Accepted
    } else {
        HostKeyCheck::Changed
    };
    Ok((read.address, check))
}
