//! Everything the tunnel does before the WebSocket upgrade, after the ticket is redeemed:
//! load the device and check its protocol is still on, resolve and check its host (R-032),
//! load the accepted RDP server certificate and check the device still presents it, load the
//! stored password for the host just resolved (SEC-011), build the `connect` values and run
//! the guacd handshake (K4 Sections 2 and 3). A password entered for another host ends the
//! session before guacd is reached.
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

use guac_protocol::ConnectParams;
use tokio::time::timeout;

mod not_opened;

pub(super) use not_opened::NotOpened;

use super::{
    arguments::{Sources, audio, connect_values, pinned_security, time_zone},
    guacd::{self, Connected},
    policy::Policy,
    route::Route,
};
use crate::{
    fingerprint::CertificateFingerprint,
    fleet::{
        credentials::{self, Secret},
        devices,
        devices::{Device, RdpSecurity},
        sessions::{
            Desktop, PendingTicket,
            connection_test::{ReadError, Reply, TLS_PROTOCOLS, read_server_certificate},
            start::{certificate_required, protocol_off},
        },
        ssh::{self, SshError, Target},
        targets::{self, AddressPolicy, LinkPort, TargetError},
        trust::{self, ServerCertificate},
    },
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
    /// The port of the hub that carries guacd to a computer on the link, for the life of
    /// the session (HMD-048); none for a device that guacd reaches by the network.
    pub(super) carried: Option<LinkPort>,
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
    let checked = Checked {
        device: &device,
        address: SocketAddr::new(address, port),
        certificate: certificate.as_ref(),
    };
    through_gateway(route, ticket, desktop, checked).await
}

/// A device whose address the opening resolved and checked, with its accepted certificate.
struct Checked<'a> {
    device: &'a Device,
    address: SocketAddr,
    certificate: Option<&'a ServerCertificate>,
}

/// Opens guacd for the device at its checked address. guacd cannot reach a computer
/// on the link, so for a link address it gets a port of the hub that carries there (HMD-048);
/// the port stays open with the session.
async fn through_gateway(
    route: &Route,
    ticket: &PendingTicket,
    desktop: Desktop,
    checked: Checked<'_>,
) -> Result<Opening, NotOpened> {
    let Checked {
        device,
        address: device_port,
        certificate,
    } = checked;
    let carried = LinkPort::open(route.live.policy(), device_port)
        .await
        .map_err(|_| NotOpened::Unreachable)?;
    let for_guacd = carried.as_ref().map_or(device_port, LinkPort::local);
    let password = password(route, device, desktop).await?;
    let sources = Sources {
        device,
        target: for_guacd.ip(),
        port: for_guacd.port(),
        password: password.as_ref(),
        certificate,
        drive_root: &route.drive_root,
    };
    let values = connect_values(desktop, &sources)?;
    drop(password);
    let params = ConnectParams {
        display: ticket.display,
        timezone: time_zone(ticket.timezone.as_deref()),
        name: ticket.operator.name.clone(),
        audio: audio(desktop, device),
        values,
    };
    let policy = Policy::of(desktop, device);
    let connected = guacd::connect(&route.guacd, desktop.select(), params, route.timing).await?;
    Ok(Opening::Gateway(Box::new(Opened {
        connected,
        policy,
        carried,
    })))
}

/// The device's stored password for `desktop`, opened only for the device's host the opening
/// resolved (SEC-011); a password entered for another host ends the opening before guacd.
async fn password(
    route: &Route,
    device: &Device,
    desktop: Desktop,
) -> Result<Option<Secret>, NotOpened> {
    let bound = (device.id, device.host.as_str());
    credentials::load_password(&route.db, &route.vault, bound, desktop.password())
        .await
        .map_err(|error| match error {
            credentials::Error::EnteredForOtherHost => NotOpened::CredentialMoved,
            other => NotOpened::Credentials(other),
        })
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
    // A computer on the link has no network address to wake; its link answers for it.
    let answered = stream.network_peer().ok_or(NotOpened::NotAwake)?;
    Ok(answered.to_canonical())
}
