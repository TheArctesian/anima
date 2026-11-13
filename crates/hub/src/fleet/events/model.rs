//! What the event log and the audit log record (LOG-004, LOG-005).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::net::IpAddr;

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::actor::{Actor, ActorKind, TokenKind};
use crate::{
    ids::{
        ApiKeyId, BuildJobId, BuilderId, ChannelId, CheckId, DeviceId, EventId, OperatorId,
        ProjectId, ReleaseId, SessionId, SiteId, SourceAccountId, WindowId,
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
    // The VPN reader's key and node warnings (VPN-010, VPN-011).
    Vpn,
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
            Self::Vpn => "vpn",
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
            Self::Vpn,
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
    /// The kind of the request that caused it; `Hub` for every other event (LOG-013).
    pub actor: ActorKind,
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
    /// `None` for an event written before LOG-013.
    pub actor: Option<ActorKind>,
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

/// The operator a recorded action is made for (API-003), and who did it for them (LOG-012).
/// A request's operator comes only from `access`, which decides `actor` (LOG-014); the hub's
/// own work uses [`Operator::hub`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operator {
    pub id: OperatorId,
    pub name: String,
    pub actor: Actor,
}

impl Operator {
    /// The name a Hub entry records as its operator: the hub's own work is done for no
    /// operator, so the entry names Anima with the nil id (OD-15).
    pub const HUB_NAME: &'static str = "Anima";

    /// The operator of an action the hub did by itself (LOG-014 rule 1).
    #[must_use]
    pub fn hub() -> Self {
        Self {
            id: OperatorId::from(Uuid::nil()),
            name: Self::HUB_NAME.to_owned(),
            actor: Actor::hub(),
        }
    }

    /// The same operator, for an action the hub does by itself on their behalf, such as
    /// ending their session at a timeout or at shutdown (LOG-014 rule 1).
    #[must_use]
    pub fn by_hub(self) -> Self {
        Self {
            actor: Actor::hub(),
            ..self
        }
    }
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
    /// A queued build job the operator stopped (BLD-008).
    Cancel,
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
            Self::Cancel => "cancel",
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
    /// A release and its build jobs (BLD-082).
    Release(ReleaseId),
    /// One build job of a release (BLD-082).
    BuildJob(BuildJobId),
    /// A computer enrolled to build releases (BLD-082).
    Builder(BuilderId),
    /// An API key for scripts (D-23).
    ApiKey(ApiKeyId),
    /// The VPN: a key, a registration or a route made through the broker (VP-7). It has no
    /// id of its own; the change names the site or node.
    Vpn,
    /// A connection to an outside source (SRC-003).
    SourceAccount(SourceAccountId),
    /// A project, its repositories or its resources (PRO-001).
    Project(ProjectId),
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
            Self::Release(_) => "release",
            Self::BuildJob(_) => "build_job",
            Self::Builder(_) => "builder",
            Self::ApiKey(_) => "api_key",
            Self::Vpn => "vpn",
            Self::SourceAccount(_) => "source_account",
            Self::Project(_) => "project",
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
            Self::Release(id) => id.to_string(),
            Self::BuildJob(id) => id.to_string(),
            Self::Builder(id) => id.to_string(),
            Self::ApiKey(id) => id.to_string(),
            Self::Vpn => "vpn".to_owned(),
            Self::SourceAccount(id) => id.to_string(),
            Self::Project(id) => id.to_string(),
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
    /// `None` for an entry written before LOG-012.
    pub(super) actor_kind: Option<ActorKind>,
    pub(super) token_kind: Option<TokenKind>,
    pub(super) api_key_name: Option<String>,
}
