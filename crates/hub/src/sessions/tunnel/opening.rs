//! Everything the tunnel does before the WebSocket upgrade, after the ticket is redeemed:
//! load the device, resolve and check its host (R-032), read its host key with the SSH probe
//! (R-034), build the `connect` values and run the guacd handshake (K4 Section 3).

use std::net::{IpAddr, SocketAddr};

use guac_protocol::{ConnectParams, DecodeError};
use tokio::time::timeout;

use super::{
    arguments::{BadConnectValue, connect_values, time_zone},
    guacd::{self, Connected},
    route::Route,
};
use crate::{
    credentials, devices,
    sessions::{
        self, PendingTicket,
        probe::{self, PROBE_TIMEOUT},
        ssh::{self, SshError, Target},
        target::TargetRule,
    },
};

/// What the tunnel opens after the upgrade.
pub(super) enum Opening {
    /// guacd is past `ready`: relay.
    Gateway(Connected),
    /// The device presented another host key than the accepted one: tell the client, close
    /// 4001. guacd was never reached and no credential left the hub (SEC-007).
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
    HostKeyUnread(#[source] SshError),
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
                 address, or to one in a network the hub refuses (ANIMA_PROBE_DENY), which the \
                 gateway must not reach."
            }
            Self::HostKeyUnread(_) => "The hub could not read the device's SSH host key.",
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
/// Safe to drop at any await: the credential leaves the hub only in the handshake's
/// `connect`, after guacd's `args`, so dropping this earlier sends it nowhere (SEC-002).
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
    let rule = route.live.target_rule();
    let port = device.ssh.port;
    let address = timeout(route.timing.connect, target(&device.host, port, rule))
        .await
        .map_err(|_| NotOpened::Unresolved)??;
    // The probe goes to the checked address, not the name, so it reads the key of the host
    // guacd will reach; it applies the rule to that address again.
    let probed = Target {
        host: &address.to_string(),
        port,
        rule,
    };
    let presented =
        probe::read_host_key(probed, PROBE_TIMEOUT)
            .await
            .map_err(|error| match error {
                SshError::RefusedTarget => NotOpened::RefusedTarget,
                other => NotOpened::HostKeyUnread(other),
            })?;
    if presented != accepted.public_key {
        return Ok(Opening::HostKeyChanged);
    }
    let secret = credentials::load_ssh(&route.db, &route.vault, device.id)
        .await
        .map_err(NotOpened::Credentials)?;
    let values = connect_values(&device, address, secret.as_ref(), &accepted)?;
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

/// The address the session goes to: the first one the device's host resolves to, when `rule`
/// lets the gateway reach every one of them (R-030, R-032), in its canonical form, so an
/// IPv4-mapped address is checked and sent as the IPv4 address it is (`ssh::addresses`).
async fn target(host: &str, port: u16, rule: &TargetRule) -> Result<IpAddr, NotOpened> {
    let checked =
        ssh::addresses(Target { host, port, rule })
            .await
            .map_err(|error| match error {
                SshError::RefusedTarget => NotOpened::RefusedTarget,
                _ => NotOpened::Unresolved,
            })?;
    checked
        .first()
        .map(SocketAddr::ip)
        .ok_or(NotOpened::Unresolved)
}
