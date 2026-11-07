//! Everything the tunnel does before the WebSocket upgrade, after the ticket is redeemed:
//! load the device and check its protocol is still on, resolve and check its host (R-032),
//! load the accepted RDP server certificate and the stored password, build the `connect`
//! values and run the guacd handshake (K4 Sections 2 and 3).

use std::net::{IpAddr, SocketAddr};

use guac_protocol::{ConnectParams, DecodeError};
use tokio::time::timeout;

use super::{
    arguments::{BadConnectValue, Desktop, Sources, audio, connect_values, time_zone},
    guacd::{self, Connected},
    route::Route,
};
use crate::{
    credentials, devices,
    devices::Device,
    sessions::{
        self, PendingTicket, Protocol, ServerCertificate,
        ssh::{self, SshError, Target},
        start::{certificate_required, protocol_off},
        target::TargetRule,
    },
};

/// What the tunnel opens after the upgrade. Each way the opening can end with something to
/// tell the client, rather than a refusal, is a variant.
pub(super) enum Opening {
    /// guacd is past `ready`: relay.
    Gateway(Connected),
}

/// Why the tunnel did not open. The messages are for the process log; none holds a value,
/// an address, guacd's text or the token (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(super) enum NotOpened {
    #[error("the ticket is for a protocol the Guacamole tunnel does not serve")]
    NotDesktop,
    #[error("the device no longer exists")]
    DeviceGone,
    #[error("the protocol is turned off for the device")]
    ProtocolOff,
    #[error("no server certificate is accepted for the device")]
    NoCertificate,
    #[error("the device host does not resolve")]
    Unresolved,
    #[error("the device host resolves to a refused address")]
    RefusedTarget,
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
    /// The event message for a `protocol` session: what the operator can do about it, with
    /// no detail from guacd.
    pub(super) const fn message(&self, protocol: Protocol) -> &'static str {
        let rdp = matches!(protocol, Protocol::Rdp);
        match self {
            Self::NotDesktop => "The ticket was not for an RDP or VNC session.",
            Self::DeviceGone if rdp => "The device was deleted before the RDP session opened.",
            Self::DeviceGone => "The device was deleted before the VNC session opened.",
            Self::ProtocolOff if rdp => {
                "RDP was turned off for the device before the session opened."
            }
            Self::ProtocolOff => "VNC was turned off for the device before the session opened.",
            Self::NoCertificate => {
                "The device has no accepted server certificate. Test the connection and accept \
                 its certificate."
            }
            Self::Unresolved => "The device's host name does not resolve.",
            Self::RefusedTarget => {
                "The device's host resolves to a loopback, link-local, multicast or unspecified \
                 address, or to one in a network the hub refuses (ANIMA_PROBE_DENY), which the \
                 gateway must not reach."
            }
            Self::Values(_) => {
                "The device's settings or stored credential cannot be sent to the gateway."
            }
            Self::Unreachable => "The hub could not reach the gateway.",
            Self::HandshakeTimeout
            | Self::GatewayClosed
            | Self::Refused { .. }
            | Self::Broken
            | Self::Garbled(_)
                if rdp =>
            {
                "The gateway did not open the RDP session."
            }
            Self::HandshakeTimeout
            | Self::GatewayClosed
            | Self::Refused { .. }
            | Self::Broken
            | Self::Garbled(_) => "The gateway did not open the VNC session.",
            Self::Devices(_) | Self::Sessions(_) | Self::Credentials(_) => {
                "The hub could not read the session's settings from its database."
            }
        }
    }
}

/// Opens the gateway side of the ticket's session. Safe to drop at any await: the
/// password leaves the hub only in the handshake's `connect`, after guacd's `args`, so
/// dropping this earlier sends it nowhere (SEC-002).
///
/// # Errors
/// [`NotOpened`] for each step that fails; the caller ends the session `GATEWAY_ERROR`.
pub(super) async fn open(route: &Route, ticket: &PendingTicket) -> Result<Opening, NotOpened> {
    let desktop = Desktop::of(ticket.protocol).ok_or(NotOpened::NotDesktop)?;
    let device = devices::get(&route.db, ticket.device)
        .await
        .map_err(|error| match error {
            devices::Error::NotFound(_) => NotOpened::DeviceGone,
            other => NotOpened::Devices(other),
        })?;
    if protocol_off(&device, ticket.protocol).is_some() {
        return Err(NotOpened::ProtocolOff);
    }
    let certificate = certificate(route, &device, desktop).await?;
    let rule = route.live.target_rule();
    let port = desktop.port(&device);
    let address = timeout(route.timing.connect, target(&device.host, port, rule))
        .await
        .map_err(|_| NotOpened::Unresolved)??;
    let password =
        credentials::load_password(&route.db, &route.vault, device.id, desktop.password())
            .await
            .map_err(NotOpened::Credentials)?;
    let sources = Sources {
        device: &device,
        target: address,
        password: password.as_ref(),
        certificate: certificate.as_ref(),
        drive_root: &route.drive_root,
    };
    let values = connect_values(desktop, &sources)?;
    drop(password);
    let params = ConnectParams {
        display: ticket.display,
        timezone: time_zone(ticket.timezone.as_deref()),
        name: ticket.operator.name.clone(),
        audio: audio(desktop, &device),
        values,
    };
    guacd::connect(&route.guacd, desktop.select(), params, route.timing)
        .await
        .map(Opening::Gateway)
}

/// The device's accepted RDP server certificate, when it has one; an RDP session that needs
/// one and has none is refused, as `start` refuses it (K4 Section 2). VNC has none.
async fn certificate(
    route: &Route,
    device: &Device,
    desktop: Desktop,
) -> Result<Option<ServerCertificate>, NotOpened> {
    if desktop == Desktop::Vnc {
        return Ok(None);
    }
    let accepted = sessions::server_certificates(&route.db, &[device.id])
        .await
        .map_err(NotOpened::Sessions)?
        .remove(&device.id);
    if accepted.is_none() && certificate_required(&device.rdp) {
        return Err(NotOpened::NoCertificate);
    }
    Ok(accepted)
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
