//! Everything the tunnel does before the WebSocket upgrade, after the ticket is redeemed:
//! load the device and check its protocol is still on, resolve and check its host (R-032),
//! load the accepted RDP server certificate and check the device still presents it, load the
//! stored password, build the `connect` values and run the guacd handshake (K4 Sections 2
//! and 3).
//!
//! The certificate check (SEC-007, K4 Section 3): for RDP with TLS or NLA and certificates
//! not ignored, the hub reads the server certificate itself at the checked address, as the
//! connection test does, before guacd is reached and before the password is loaded; guacd
//! reports a mismatch only as a generic error. A different certificate opens nothing at guacd
//! and ends the session `SERVER_CERTIFICATE_MISMATCH`. When the setting is `ANY`, guacd gets
//! the TLS security the server chose instead (`pinned_security`), so its connection cannot
//! fall back to standard RDP security, where no certificate is checked. guacd pins
//! `cert-fingerprints` on that TLS connection, so a certificate that changes between the hub's
//! read and guacd's connect fails closed as `GATEWAY_ERROR`. The read is not kept as an
//! observation: only a connection test lets the operator accept a certificate.
//!
//! When the ticket's start sent a Wake-on-LAN packet, the opening first waits, up to the
//! woken limit, until the device's port answers, so guacd connects to a device that is up
//! (K4 Section 1). guacd's own connection to the device is not the hub's to retry.

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use guac_protocol::{ConnectParams, DecodeError};
use tokio::time::timeout;

use super::{
    arguments::{BadConnectValue, Sources, audio, connect_values, pinned_security, time_zone},
    guacd::{self, Connected},
    policy::Policy,
    route::Route,
};
use crate::{
    credentials, devices,
    devices::{Device, RdpSecurity},
    sessions::{
        self, Desktop, PendingTicket, Protocol,
        connection_test::{ReadError, Reply, TLS_PROTOCOLS, read_server_certificate},
        ssh::{self, SshError, Target},
        start::{certificate_required, protocol_off},
    },
    targets::{self, AddressPolicy, TargetError},
    trust::{self, CertificateFingerprint, ServerCertificate},
};

/// What the tunnel opens after the upgrade. Each way the opening can end with something to
/// tell the client, rather than a refusal, is a variant.
pub(super) enum Opening {
    /// guacd is past `ready`: relay. Boxed, as it is far larger than the other variant.
    Gateway(Box<Opened>),
    /// The device presented another RDP server certificate than the accepted one; guacd was
    /// never reached.
    CertificateChanged,
}

/// guacd past `ready`, and what the device lets the session move, read with the rest of the
/// device record so it holds for the session's life (K4 Section 4).
pub(super) struct Opened {
    pub(super) connected: Connected,
    pub(super) policy: Policy,
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
    #[error("the device did not answer in time after the Wake-on-LAN packet")]
    NotAwake,
    #[error("the device's RDP server certificate could not be read")]
    CertificateUnread(#[source] ReadError),
    #[error("the device offers only standard RDP security, so it has no certificate")]
    StandardOnly,
    #[error("the device refused the RDP security the hub asked for to read its certificate")]
    SecurityRefused,
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
            Self::NotAwake => {
                "The device did not answer within 120 seconds of the Wake-on-LAN packet."
            }
            Self::CertificateUnread(error) => unread_message(*error),
            Self::StandardOnly => {
                "The device offers only standard RDP security, so the hub could not check its \
                 server certificate. Check the security setting."
            }
            Self::SecurityRefused => {
                "The device refused the RDP security the hub asked for to check its server \
                 certificate. Check the security setting."
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

/// The event message for a server certificate the hub could not read from the device.
const fn unread_message(error: ReadError) -> &'static str {
    match error {
        ReadError::Unexpected => {
            "The device did not answer as an RDP server when the hub checked its server \
             certificate."
        }
        ReadError::Tls => {
            "The device did not complete a TLS handshake when the hub checked its server \
             certificate."
        }
        _ => "The hub could not reach the device's RDP port to check its server certificate.",
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
    let mut device = devices::get(&route.db, ticket.device)
        .await
        .map_err(|error| match error {
            devices::Error::NotFound(_) => NotOpened::DeviceGone,
            other => NotOpened::Devices(other),
        })?;
    if protocol_off(&device, ticket.protocol).is_some() {
        return Err(NotOpened::ProtocolOff);
    }
    let certificate = certificate(route, &device, desktop).await?;
    let policy = route.live.policy();
    let port = desktop.port(&device);
    let address = if ticket.wake_sent {
        let awake = awake(&device.host, port, policy, route.timing.reconnect);
        timeout(route.timing.woken_opening, awake)
            .await
            .map_err(|_| NotOpened::NotAwake)??
    } else {
        timeout(route.timing.connect, target(&device.host, port, policy))
            .await
            .map_err(|_| NotOpened::Unresolved)??
    };
    let pinned = certificate
        .as_ref()
        .filter(|_| certificate_required(&device.rdp));
    if let Some(accepted) = pinned {
        let Some(chosen) = presents(route, SocketAddr::new(address, port), accepted).await? else {
            return Ok(Opening::CertificateChanged);
        };
        device.rdp.security = pinned_security(device.rdp.security, chosen);
    }
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
    let policy = Policy::of(desktop, &device);
    guacd::connect(&route.guacd, desktop.select(), params, route.timing)
        .await
        .map(|connected| Opening::Gateway(Box::new(Opened { connected, policy })))
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
    let accepted = trust::server_certificates(&route.db, &[device.id])
        .await
        .map_err(|error| NotOpened::Sessions(error.into()))?
        .remove(&device.id);
    if accepted.is_none() && certificate_required(&device.rdp) {
        return Err(NotOpened::NoCertificate);
    }
    Ok(accepted)
}

/// The TLS security the RDP server at the checked `address` chose when it presents
/// `accepted`; `None` when it presents another certificate. The hub asks for TLS or NLA, as
/// the connection test does, and reads the certificate within the connect wait. The address
/// is an IP, so it is not resolved again; the address policy still checks it.
async fn presents(
    route: &Route,
    address: SocketAddr,
    accepted: &ServerCertificate,
) -> Result<Option<RdpSecurity>, NotOpened> {
    let host = address.ip().to_string();
    let target = Target {
        host: &host,
        port: address.port(),
        policy: route.live.policy(),
    };
    let read = read_server_certificate(target, TLS_PROTOCOLS, route.timing.connect).await;
    match read.map_err(|error| match error {
        ReadError::Unresolved => NotOpened::Unresolved,
        ReadError::RefusedTarget => NotOpened::RefusedTarget,
        other => NotOpened::CertificateUnread(other),
    })? {
        Reply::Certificate(presented, chosen) => Ok(accepted
            .fingerprint_sha256
            .parse::<CertificateFingerprint>()
            .is_ok_and(|accepted| accepted == presented.fingerprint)
            .then_some(chosen)),
        Reply::Standard => Err(NotOpened::StandardOnly),
        Reply::Refused => Err(NotOpened::SecurityRefused),
    }
}

/// The address the session goes to: the first one the device's host resolves to, when
/// `policy` lets the gateway reach every one of them (R-030, R-032), in its canonical form,
/// so an IPv4-mapped address is checked and sent as the IPv4 address it is
/// ([`targets::resolve_checked`]).
async fn target(host: &str, port: u16, policy: &AddressPolicy) -> Result<IpAddr, NotOpened> {
    let checked =
        targets::resolve_checked(host, port, policy)
            .await
            .map_err(|error| match error {
                TargetError::Refused => NotOpened::RefusedTarget,
                TargetError::Unresolved => NotOpened::Unresolved,
            })?;
    checked
        .first()
        .map(SocketAddr::ip)
        .ok_or(NotOpened::Unresolved)
}

/// The address of the device that is waking up, once its port answers: the first of its
/// checked addresses that accepts a connection, tried again every `retry`. The
/// connection is closed at once; guacd makes its own.
async fn awake(
    host: &str,
    port: u16,
    policy: &AddressPolicy,
    retry: Duration,
) -> Result<IpAddr, NotOpened> {
    let stream = ssh::connect_waking(Target { host, port, policy }, retry)
        .await
        .map_err(|error| match error {
            SshError::RefusedTarget => NotOpened::RefusedTarget,
            _ => NotOpened::Unresolved,
        })?;
    let answered = stream.peer_addr().map_err(|_| NotOpened::NotAwake)?;
    Ok(answered.ip().to_canonical())
}
