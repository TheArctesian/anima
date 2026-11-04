//! Domain types of remote sessions (RA-008) and accepted host keys (SEC-006).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::{net::IpAddr, str::FromStr};

use async_graphql::Enum;
use chrono::{DateTime, Utc};

use super::public_key::PublicKey;
use crate::ids::{DeviceId, OperatorId, SessionId};

/// The database holds text that is not a known value of an enum.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown {kind} '{text}'")]
pub struct UnknownValue {
    kind: &'static str,
    text: String,
}

/// The remote protocol of a session, stored as lower-case text. Phase 1 has SSH only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Protocol {
    Ssh,
}

impl Protocol {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
        }
    }
}

impl FromStr for Protocol {
    type Err = UnknownValue;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "ssh" => Ok(Self::Ssh),
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
    Timeout,
    HubShutdown,
}

impl EndReason {
    /// Every reason, in K1 order.
    pub const ALL: [Self; 7] = [
        Self::Operator,
        Self::RemoteClosed,
        Self::GatewayError,
        Self::AuthFailed,
        Self::HostKeyMismatch,
        Self::Timeout,
        Self::HubShutdown,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::RemoteClosed => "remote_closed",
            Self::GatewayError => "gateway_error",
            Self::AuthFailed => "auth_failed",
            Self::HostKeyMismatch => "host_key_mismatch",
            Self::Timeout => "timeout",
            Self::HubShutdown => "hub_shutdown",
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

/// One query argument the operator must fix. `field` is the GraphQL argument name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    pub field: &'static str,
    pub message: String,
}

impl FieldError {
    pub(super) fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}
