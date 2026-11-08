//! Domain types of remote sessions (RA-008), accepted host keys (SEC-006) and accepted RDP
//! server certificates (SEC-007 for RDP).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::{net::IpAddr, str::FromStr};

use async_graphql::Enum;
use chrono::{DateTime, Utc};

use super::public_key::PublicKey;
use crate::ids::{DeviceId, OperatorId, SessionId};

pub use crate::api_errors::FieldError;

/// The database holds text that is not a known value of an enum.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown {kind} '{text}'")]
pub struct UnknownValue {
    kind: &'static str,
    text: String,
}

/// The remote protocol of a session, stored as lower-case text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Protocol {
    Ssh,
    Rdp,
    Vnc,
}

impl Protocol {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::Rdp => "rdp",
            Self::Vnc => "vnc",
        }
    }

    /// Where the client opens this protocol's tunnel (K4 Section 1 `tunnelPath`): SSH uses
    /// the terminal tunnel (K4 Section 8, R-062); RDP and VNC use the Guacamole tunnel.
    #[must_use]
    pub const fn tunnel_path(self) -> &'static str {
        match self {
            Self::Ssh => "/terminal",
            Self::Rdp | Self::Vnc => "/tunnel",
        }
    }
}

/// A desktop protocol: what the Guacamole tunnel serves and the RDP and VNC connection test
/// reads (K4 Section 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Desktop {
    Rdp,
    Vnc,
}

impl Desktop {
    /// The desktop protocol of a session's `protocol`; `None` for SSH, which uses the terminal
    /// tunnel (K4 Section 8).
    pub(super) const fn of(protocol: Protocol) -> Option<Self> {
        match protocol {
            Protocol::Rdp => Some(Self::Rdp),
            Protocol::Vnc => Some(Self::Vnc),
            Protocol::Ssh => None,
        }
    }
}

impl FromStr for Protocol {
    type Err = UnknownValue;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "ssh" => Ok(Self::Ssh),
            "rdp" => Ok(Self::Rdp),
            "vnc" => Ok(Self::Vnc),
            _ => Err(UnknownValue {
                kind: "protocol",
                text: text.to_owned(),
            }),
        }
    }
}

/// Why a session ended (K1 `SessionEndReason`), stored as `lower_snake` text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "SessionEndReason")]
pub enum EndReason {
    Operator,
    RemoteClosed,
    GatewayError,
    AuthFailed,
    HostKeyMismatch,
    /// The client sent an instruction that does not decode (GW-011, R-034).
    ProtocolError,
    Timeout,
    HubShutdown,
    /// The RDP certificate differs from the accepted one (SEC-007).
    ServerCertificateMismatch,
}

impl EndReason {
    /// Every reason, in K1 order.
    pub const ALL: [Self; 9] = [
        Self::Operator,
        Self::RemoteClosed,
        Self::GatewayError,
        Self::AuthFailed,
        Self::HostKeyMismatch,
        Self::ProtocolError,
        Self::Timeout,
        Self::HubShutdown,
        Self::ServerCertificateMismatch,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::RemoteClosed => "remote_closed",
            Self::GatewayError => "gateway_error",
            Self::AuthFailed => "auth_failed",
            Self::HostKeyMismatch => "host_key_mismatch",
            Self::ProtocolError => "protocol_error",
            Self::Timeout => "timeout",
            Self::HubShutdown => "hub_shutdown",
            Self::ServerCertificateMismatch => "server_certificate_mismatch",
        }
    }
}

impl FromStr for EndReason {
    type Err = UnknownValue;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|reason| reason.as_str() == text)
            .ok_or_else(|| UnknownValue {
                kind: "session end reason",
                text: text.to_owned(),
            })
    }
}

/// When and why a session ended. Both or neither are stored (`sessions_end_pair_check`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionEnd {
    pub at: DateTime<Utc>,
    pub reason: EndReason,
}

/// One remote session, live or ended (RA-008). Names are copies, so the record outlives the
/// device and the operator account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub id: SessionId,
    pub device: Option<DeviceId>,
    pub device_name: String,
    pub protocol: Protocol,
    pub operator: OperatorId,
    pub operator_name: String,
    pub client: IpAddr,
    pub started_at: DateTime<Utc>,
    /// `None` while the session is live.
    pub end: Option<SessionEnd>,
}

impl Session {
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.end.is_none()
    }
}

/// A session's start or end, as every client sees it live (RA-034): the row as it was
/// committed, which holds no session content, token or gateway ID (RA-008).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionChange {
    pub session: Session,
}

/// Which sessions `Query.sessions` lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionFilter {
    pub device: Option<DeviceId>,
    /// `Some(true)`: only live sessions; `Some(false)`: only ended ones.
    pub live: Option<bool>,
}

/// The host key the operator accepted for a device (SEC-006). Its algorithm and fingerprint
/// are read from the key itself, so they always agree with the key the hub pins (SEC-007).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostKey {
    pub public_key: PublicKey,
    /// When the hub first saw this key.
    pub first_seen_at: DateTime<Utc>,
}

/// The RDP server certificate the operator accepted for a device (SEC-007). Every field is what
/// the device's leaf certificate said when the hub read it; only the fingerprint is pinned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerCertificate {
    /// SHA-256 of the leaf certificate's DER: 32 lower-case hex pairs joined by colons, as
    /// `FreeRDP` writes it (`server_certificates_fingerprint_check`).
    pub fingerprint_sha256: String,
    /// The certificate's subject, as the device chose it; never trusted.
    pub subject: Option<String>,
    pub not_after: Option<DateTime<Utc>>,
    /// When the hub first saw this certificate.
    pub first_seen_at: DateTime<Utc>,
}
