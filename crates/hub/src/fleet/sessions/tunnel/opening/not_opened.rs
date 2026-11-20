//! Why the tunnel did not open, and what the operator is told about it.

use guac_protocol::DecodeError;

use super::super::arguments::BadConnectValue;
use crate::fleet::{
    credentials, devices,
    sessions::{self, Protocol, connection_test::ReadError},
};

/// Why the tunnel did not open. The messages are for the process log; none holds a value,
/// an address, guacd's text or the token (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(in crate::fleet::sessions::tunnel) enum NotOpened {
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
    /// The stored password was entered for another host than the device's host now; guacd was
    /// never reached (SEC-011).
    #[error("the stored password was entered for another host of the device")]
    CredentialMoved,
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
    pub(in crate::fleet::sessions::tunnel) const fn message(
        &self,
        protocol: Protocol,
    ) -> &'static str {
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
            Self::CredentialMoved => sessions::credential_moved(protocol),
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
