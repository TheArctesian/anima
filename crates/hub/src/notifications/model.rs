//! What the notification store holds and returns (K2 `notifications` tables, K1 types).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ids::{ChannelId, CheckId, DeliveryId, DeviceId},
    status::Status,
};

/// How a channel reaches the operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChannelKind {
    Webhook,
    Ntfy,
    Email,
    Apprise,
}

impl ChannelKind {
    /// The text stored in `channels.kind`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Webhook => "webhook",
            Self::Ntfy => "ntfy",
            Self::Email => "email",
            Self::Apprise => "apprise",
        }
    }

    /// The kind whose stored text is `text`.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        [Self::Webhook, Self::Ntfy, Self::Email, Self::Apprise]
            .into_iter()
            .find(|kind| kind.as_str() == text)
    }
}

/// What a notification says happened (K1 `NotificationKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotificationKind {
    Down,
    Up,
    StillDown,
    CertificateExpiry,
    DomainExpiry,
    Test,
}

impl NotificationKind {
    /// The text stored in `outbox.kind` and `deliveries.kind`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::Up => "up",
            Self::StillDown => "still_down",
            Self::CertificateExpiry => "certificate_expiry",
            Self::DomainExpiry => "domain_expiry",
            Self::Test => "test",
        }
    }

    /// The kind whose stored text is `text`.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        [
            Self::Down,
            Self::Up,
            Self::StillDown,
            Self::CertificateExpiry,
            Self::DomainExpiry,
            Self::Test,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == text)
    }
}

/// The fields of a channel the operator sets. `secrets` is already sealed: the store never sees
/// plaintext (SEC-003).
#[derive(Clone, PartialEq, Eq)]
pub struct ChannelFields {
    pub name: String,
    pub kind: ChannelKind,
    pub on_by_default: bool,
    /// The kind's non-secret fields.
    pub settings: serde_json::Value,
    pub secrets: Option<Vec<u8>>,
    pub title_template: Option<String>,
    pub body_template: Option<String>,
}

/// A stored channel. No `Debug`: `secrets` is sealed key material.
#[derive(Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: ChannelId,
    pub fields: ChannelFields,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What an expiry notification is about (D-22). Marks of the two never mix.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpirySubject {
    #[default]
    Certificate,
    Domain,
}

/// A threshold already announced for one certificate or domain, keyed by its `notAfter` (D-22).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpiryMark {
    /// Rows written before domain expiry existed have none and mean the certificate.
    #[serde(default)]
    pub subject: ExpirySubject,
    pub threshold_days: u16,
    pub not_after: DateTime<Utc>,
}

/// What the decision core remembers about a check between beats (D-16).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckState {
    /// The status last announced; `None` before the first announcement.
    pub notified_status: Option<Status>,
    /// Down beats since the last Down or Still-down notification.
    pub down_beats: u32,
    pub expiry_marks: Vec<ExpiryMark>,
}

/// How a check's notifications behave (NOT-005).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheckSettings {
    /// Down beats between repeated notifications; 0 means never. At most 1000.
    pub resend_every_beats: u16,
}

/// The id of an outbox row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct OutboxId(Uuid);

impl From<Uuid> for OutboxId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// A notification to queue for one channel (D-16). `payload` holds rendered placeholder values
/// and never a secret.
#[derive(Clone, Debug, PartialEq)]
pub struct NewOutbox {
    pub id: OutboxId,
    pub channel: ChannelId,
    pub check: Option<CheckId>,
    pub kind: NotificationKind,
    pub payload: serde_json::Value,
}

/// A queued notification the dispatcher has claimed.
#[derive(Clone, Debug, PartialEq)]
pub struct Outbox {
    pub id: OutboxId,
    pub channel: ChannelId,
    pub check: Option<CheckId>,
    pub kind: NotificationKind,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    /// Tries so far, this claim's included: each claim counts one (D-16).
    pub attempts: u32,
    /// When this claim's lease ends: the `next_attempt_at` the claim set. It is the claim's
    /// token: an outcome changes the row only while the row still carries it, so a claimer
    /// whose lease ended changes nothing (D-16).
    pub lease_end: DateTime<Utc>,
}

/// One send, or one failure to send, as the delivery log keeps it (NOT-004).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery {
    pub id: DeliveryId,
    /// The id the channel had when this was written. The log keeps no foreign key (DB-004), so
    /// the channel may be deleted since: `channel_name` is the copy kept.
    pub channel: Option<ChannelId>,
    pub channel_name: String,
    pub kind: NotificationKind,
    pub at: DateTime<Utc>,
    pub ok: bool,
    /// Why it failed; `None` exactly when `ok`.
    pub reason: Option<DeliveryReason>,
    /// The HTTP status or SMTP reply code, when the reason has one. Never the reply text
    /// (LOG-007).
    pub code: Option<i32>,
}

/// Why a send failed, as a fixed word (K1 `DeliveryReason`, CR-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeliveryReason {
    TargetNotPermitted,
    BadAddress,
    SecretUnreadable,
    SetupFailed,
    ConnectionFailed,
    TlsFailed,
    TimedOut,
    HttpStatus,
    NoStarttls,
    SmtpRefused,
    NotUnderstood,
}

impl DeliveryReason {
    const ALL: [Self; 11] = [
        Self::TargetNotPermitted,
        Self::BadAddress,
        Self::SecretUnreadable,
        Self::SetupFailed,
        Self::ConnectionFailed,
        Self::TlsFailed,
        Self::TimedOut,
        Self::HttpStatus,
        Self::NoStarttls,
        Self::SmtpRefused,
        Self::NotUnderstood,
    ];

    /// The text stored in `deliveries.reason`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TargetNotPermitted => "target_not_permitted",
            Self::BadAddress => "bad_address",
            Self::SecretUnreadable => "secret_unreadable",
            Self::SetupFailed => "setup_failed",
            Self::ConnectionFailed => "connection_failed",
            Self::TlsFailed => "tls_failed",
            Self::TimedOut => "timed_out",
            Self::HttpStatus => "http_status",
            Self::NoStarttls => "no_starttls",
            Self::SmtpRefused => "smtp_refused",
            Self::NotUnderstood => "not_understood",
        }
    }

    /// The reason whose stored text is `text`.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.as_str() == text)
    }
}

/// Where a page of deliveries starts: after this delivery, newest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeliveryCursor {
    pub at: DateTime<Utc>,
    pub id: DeliveryId,
}

/// The installation's notification settings (D-21, D-22).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallationSettings {
    /// Days before expiry at which to notify: 1 to 5 values of 1 to 365, descending.
    pub certificate_days: Vec<u16>,
    pub title_template: String,
    pub body_template: String,
}

/// The status as the stored text of `check_state.notified_status`.
pub(super) const fn status_text(status: Status) -> &'static str {
    match status {
        Status::Up => "up",
        Status::Down => "down",
        Status::Degraded => "degraded",
        Status::Maintenance => "maintenance",
        Status::Unknown => "unknown",
    }
}

/// The status whose stored text is `text`.
pub(super) fn status_from_stored(text: &str) -> Option<Status> {
    [
        Status::Up,
        Status::Down,
        Status::Degraded,
        Status::Maintenance,
        Status::Unknown,
    ]
    .into_iter()
    .find(|status| status_text(*status) == text)
}

/// What checks tells notifications about one heartbeat, inside the heartbeat transaction (D-16).
pub struct Observation {
    pub check: CheckId,
    pub check_name: String,
    pub check_type: &'static str,
    pub device: (DeviceId, String),
    pub site: String,
    pub previous: Status,
    pub status: Status,
    pub in_maintenance: bool,
    pub at: DateTime<Utc>,
    pub message: Option<String>,
    pub certificate_not_after: Option<DateTime<Utc>>,
    pub domain_not_after: Option<DateTime<Utc>>,
}

/// What one check notifies, for `Check.notifications` (API-005). The channels carry no sealed
/// secrets.
#[derive(Clone)]
pub struct CheckNotifications {
    pub channels: Vec<Channel>,
    pub resend_every_beats: u16,
}
