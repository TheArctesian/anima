//! What a check is to the rest of the hub, and what callers pass in to change one.

use std::{fmt, net::IpAddr};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};

use super::{
    config::{CheckConfig, CheckType, HttpInput},
    evaluation::{CheckSettings, CheckState},
    probe::ProbeResult,
};
use crate::{
    ids::{CheckId, DeviceId},
    status::Status,
};

/// A check as stored. HTTP secrets are never part of it (LOG-007).
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub id: CheckId,
    pub device: DeviceId,
    /// The group this check belongs to.
    pub parent: Option<CheckId>,
    pub name: String,
    pub config: CheckConfig,
    pub settings: CheckSettings,
    pub paused: bool,
    /// The stored state of the last result; read the shown status through the status rules.
    pub state: CheckState,
    pub last_beat_at: Option<DateTime<Utc>>,
    /// Leaf certificate expiry from the last TLS or HTTPS result that had one (CHK-007).
    pub cert_not_after: Option<DateTime<Utc>>,
    /// The address the last probe that resolved one used.
    pub resolved_ip: Option<IpAddr>,
    /// Only PUSH checks have one.
    pub push_token: Option<PushToken>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Check {
    #[must_use]
    pub const fn kind(&self) -> CheckType {
        self.config.kind()
    }
}

/// The token in a PUSH check's URL: 32 random bytes in unpadded base64url. It is the
/// only authentication of the push route, so `Debug` never prints it (LOG-007).
#[derive(Clone, PartialEq, Eq)]
pub struct PushToken(String);

impl PushToken {
    /// The token for these random bytes; the caller supplies the randomness.
    #[must_use]
    pub fn from_random(bytes: &[u8; 32]) -> Self {
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    /// The token as stored, for the push path.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub(super) const fn stored(text: String) -> Self {
        Self(text)
    }
}

impl fmt::Debug for PushToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PushToken(..)")
    }
}

/// What a check does, with the parts that are not stored in `config`.
pub enum Definition {
    /// Every type except HTTP and PUSH.
    Config(CheckConfig),
    /// The HTTP check with its secrets in hand; they are sealed apart from `config`.
    Http(HttpInput),
    /// A PUSH check with a new token.
    Push(PushToken),
}

impl Definition {
    #[must_use]
    pub const fn kind(&self) -> CheckType {
        match self {
            Self::Config(config) => config.kind(),
            Self::Http(_) => CheckType::Http,
            Self::Push(_) => CheckType::Push,
        }
    }
}

/// A check to add (the API's `CreateCheckInput`).
pub struct NewCheck {
    pub device: DeviceId,
    /// A group check on the same device.
    pub parent: Option<CheckId>,
    pub name: String,
    pub definition: Definition,
    pub settings: CheckSettings,
}

/// The fields to change on a check (the API's `UpdateCheckInput`); `None` keeps a field.
///
/// The type of a check never changes. An HTTP definition replaces the whole HTTP setup, but
/// an empty secret keeps the stored one: an empty value of a secret header keeps the stored
/// value of the header with that name in the same position, an empty basic password keeps
/// the stored password and an empty bearer token keeps the stored token.
#[derive(Default)]
pub struct CheckChange {
    /// `Some(None)` removes the check from its group.
    pub parent: Option<Option<CheckId>>,
    pub name: Option<String>,
    pub definition: Option<Definition>,
    pub interval_seconds: Option<u32>,
    pub retries: Option<u8>,
    pub retry_interval_seconds: Option<u32>,
    pub degraded_threshold_ms: Option<u32>,
    pub invert: Option<bool>,
}

/// One input field the operator must fix. `field` is the GraphQL input field name.
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

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// What a lookup could not find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Device(DeviceId),
    Check(CheckId),
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Device(id) => write!(f, "device {id}"),
            Self::Check(id) => write!(f, "check {id}"),
        }
    }
}

/// One result to record: the heartbeat (CHK-003) and the check's new state.
#[derive(Clone, Debug, PartialEq)]
pub struct Beat {
    pub at: DateTime<Utc>,
    /// The heartbeat status, as `evaluate` returned it.
    pub status: Status,
    pub response_time_ms: Option<u32>,
    /// Cleaned again when written: at most 512 characters, no control characters.
    pub message: Option<String>,
    pub state: CheckState,
    /// Kept on the check only when the probe reported one.
    pub resolved_ip: Option<IpAddr>,
    pub cert_not_after: Option<DateTime<Utc>>,
}

impl Beat {
    /// The beat for a probe result and its evaluation. The response time is kept only for an
    /// Up or Degraded beat of a probe that succeeded, so the average describes answered
    /// probes: an inverted Up (a failed probe) has none, as `evaluate` ignores it too.
    #[must_use]
    pub fn of(
        probe: ProbeResult,
        (state, status): (CheckState, Status),
        at: DateTime<Utc>,
    ) -> Self {
        let answered = probe.ok && matches!(status, Status::Up | Status::Degraded);
        let millis =
            |time: std::time::Duration| u32::try_from(time.as_millis()).unwrap_or(u32::MAX);
        Self {
            at,
            status,
            response_time_ms: probe.response_time.filter(|_| answered).map(millis),
            message: probe.message,
            state,
            resolved_ip: probe.resolved_ip,
            cert_not_after: probe.cert_not_after,
        }
    }
}

/// What `record_beat` did with a beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum Recorded {
    /// The heartbeat, its aggregates and the check's state were written.
    Written,
    /// Nothing was written: the check is paused, or the beat is not newer than its last beat.
    Skipped,
}

/// A check whose next probe is due, as the scheduler needs it.
#[derive(Clone, Debug, PartialEq)]
pub struct Due {
    pub id: CheckId,
    pub device: DeviceId,
    /// `None` when the stored configuration cannot be read; the probe then fails as a bad
    /// configuration instead of stopping the schedule.
    pub config: Option<CheckConfig>,
    pub settings: CheckSettings,
    pub state: CheckState,
}
