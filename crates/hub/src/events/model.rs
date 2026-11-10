//! What the event log and the audit log record (LOG-004, LOG-005).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::net::IpAddr;

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    ids::{
        ApiKeyId, ChannelId, CheckId, DeviceId, EventId, OperatorId, SessionId, ShareId, SiteId,
        WindowId,
    },
    status::Status,
};

/// How much an event needs the operator's attention. Ordered: `Info < Warning < Error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Enum)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    /// The text stored in `event_log.severity`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    /// The severity whose stored text is `text`.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        [Self::Info, Self::Warning, Self::Error]
            .into_iter()
            .find(|severity| severity.as_str() == text)
    }
}

/// The part of the hub that produced an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "EventSource")]
pub enum Source {
    Check,
    Session,
    Device,
    Access,
    Hub,
    Metrics,
    // The notification dispatcher (NOT-004). K1 names it `NOTIFICATION`.
    #[graphql(name = "NOTIFICATION")]
    Notifications,
}

impl Source {
    /// The text stored in `event_log.source`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Session => "session",
            Self::Device => "device",
            Self::Access => "access",
            Self::Hub => "hub",
            Self::Metrics => "metrics",
            Self::Notifications => "notification",
        }
    }

    /// The source whose stored text is `text`.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        [
            Self::Check,
            Self::Session,
            Self::Device,
            Self::Access,
            Self::Hub,
            Self::Metrics,
            Self::Notifications,
        ]
        .into_iter()
        .find(|source| source.as_str() == text)
    }
}

/// An event to record. `device` carries the device name at the time of the event, so the
/// entry keeps it after the device is renamed or deleted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewEvent {
    pub severity: Severity,
    pub source: Source,
    pub device: Option<(DeviceId, String)>,
    pub message: String,
}

/// A stored event, as subscribers and the event list see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventSummary {
    pub id: EventId,
    pub at: DateTime<Utc>,
    pub severity: Severity,
    pub source: Source,
    pub device: Option<(DeviceId, String)>,
    pub message: String,
}

/// Which events a list shows (K1 `EventFilter`). `None` lets every value through; an empty
/// list lets none through.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct EventFilter {
    pub(super) severities: Option<Vec<Severity>>,
    pub(super) sources: Option<Vec<Source>>,
    pub(super) device: Option<DeviceId>,
    /// Only events at or after this instant.
    pub(super) since: Option<DateTime<Utc>>,
}

/// The signed-in operator who made a request (API-003).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operator {
    pub id: OperatorId,
    pub name: String,
}

/// The network address the request came from (LOG-005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientAddr(pub IpAddr);

/// One check result, as the live status view receives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeartbeatSummary {
    pub at: DateTime<Utc>,
    pub status: Status,
    pub response_time_ms: Option<u32>,
    /// The cleaned message as stored.
    pub message: Option<String>,
}

/// The verb of an audit entry. Its text form is stored in `audit_log.action`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuditAction {
    Login,
    Logout,
    Create,
    Update,
    Delete,
    Start,
    End,
    Accept,
    Pause,
    Resume,
    Activate,
    Deactivate,
    /// A file sent to a device over the files tunnel (K4 Section 9).
    Upload,
    /// A file read from a device over the files tunnel (K4 Section 9).
    Download,
    /// A Wake-on-LAN packet sent to a device (RA-031).
    Wake,
    /// A test notification sent through a channel (NOT-004).
    Send,
}

impl AuditAction {
    /// Stable `lower_snake` text, as stored and shown in `AuditEntry.action`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Logout => "logout",
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Start => "start",
            Self::End => "end",
            Self::Accept => "accept",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Activate => "activate",
            Self::Deactivate => "deactivate",
            Self::Upload => "upload",
            Self::Download => "download",
            Self::Wake => "wake",
            Self::Send => "send",
        }
    }
}

/// The thing an audited action changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuditTarget {
    Operator(OperatorId),
    Site(SiteId),
    Device(DeviceId),
    Check(CheckId),
    Window(WindowId),
    Session(SessionId),
    /// The SSH credential of a device.
    Credential(DeviceId),
    /// The accepted SSH host key of a device.
    HostKey(DeviceId),
    /// A file on a device, moved over the files tunnel; the change holds its path.
    File(DeviceId),
    /// A hub setting, by its dotted name (for example `retention.events`).
    Setting(&'static str),
    /// A notification channel.
    Channel(ChannelId),
    /// A share of a session (RA-035).
    Share(ShareId),
    /// An API key for scripts (D-23).
    ApiKey(ApiKeyId),
}

impl AuditTarget {
    /// The text stored in `audit_log.target_type`.
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Operator(_) => "operator",
            Self::Site(_) => "site",
            Self::Device(_) => "device",
            Self::Check(_) => "check",
            Self::Window(_) => "maintenance_window",
            Self::Session(_) => "session",
            Self::Credential(_) => "credential",
            Self::HostKey(_) => "host_key",
            Self::File(_) => "file",
            Self::Setting(_) => "setting",
            Self::Channel(_) => "channel",
            Self::Share(_) => "share",
            Self::ApiKey(_) => "api_key",
        }
    }

    /// The UUID text stored in `audit_log.target_id`.
    #[must_use]
    pub fn id(self) -> String {
        match self {
            Self::Operator(id) => id.to_string(),
            Self::Site(id) => id.to_string(),
            Self::Device(id) | Self::Credential(id) | Self::HostKey(id) | Self::File(id) => {
                id.to_string()
            }
            Self::Check(id) => id.to_string(),
            Self::Window(id) => id.to_string(),
            Self::Session(id) => id.to_string(),
            Self::Setting(name) => name.to_owned(),
            Self::Channel(id) => id.to_string(),
            Self::Share(id) => id.to_string(),
            Self::ApiKey(id) => id.to_string(),
        }
    }
}

/// An audit entry to record. `change` lists the changed fields; `audit` redacts secret
/// values before it writes (LOG-007).
///
/// No `Debug`: `change` may hold a secret until it is redacted.
pub struct NewAudit {
    pub operator: Operator,
    pub action: AuditAction,
    pub target: AuditTarget,
    pub client: ClientAddr,
    pub change: serde_json::Value,
}

/// A stored audit entry, as the audit list shows it (K1 `AuditEntry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AuditRecord {
    pub(super) id: Uuid,
    pub(super) at: DateTime<Utc>,
    pub(super) operator_name: String,
    pub(super) action: String,
    pub(super) target_type: String,
    pub(super) target_id: String,
    pub(super) client: IpAddr,
    /// Already redacted when it was written (LOG-007).
    pub(super) change: serde_json::Value,
}
