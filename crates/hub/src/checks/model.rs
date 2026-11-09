//! What a check is to the rest of the hub, and what callers pass in to change one.

use std::{fmt, net::IpAddr};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};

use super::{
    config::{
        CheckConfig, CheckType, HttpConfig, HttpInput, HttpSecrets, SnmpConfig, SnmpInput,
        SnmpSecrets,
    },
    evaluation::{CheckSettings, CheckState},
    probe::ProbeResult,
};
use crate::{
    ids::{CheckId, DeviceId},
    status::Status,
};

pub use crate::api_errors::FieldError;

/// A check as stored. HTTP and SNMP secrets are never part of it (LOG-007).
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
    /// Registration expiry from the last RDAP reply that gave one (D-29).
    pub domain_not_after: Option<DateTime<Utc>>,
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

    /// Whole days from `now` to the certificate's expiry, rounded down, so negative once it
    /// has expired. `None` unless the check is TLS or HTTPS and has seen a certificate
    /// (CHK-007).
    #[must_use]
    pub fn certificate_days_left(&self, now: DateTime<Utc>) -> Option<i32> {
        let secure = match &self.config {
            CheckConfig::Tls { .. } => true,
            CheckConfig::Http(http) => {
                reqwest::Url::parse(&http.url).is_ok_and(|url| url.scheme() == "https")
            }
            _ => false,
        };
        let not_after = self.cert_not_after?;
        secure.then(|| whole_days(not_after, now))
    }

    /// Whole days from `now` to the domain's registration expiry, rounded down, so negative
    /// once it has expired. `None` unless the check is a domain expiry check and RDAP gave a
    /// date (D-29).
    ///
    /// The days count from the last expiry date the registry gave: a later reply without a
    /// date keeps it. The status follows the latest probe alone, so that reply is Unknown.
    #[must_use]
    pub fn domain_days_left(&self, now: DateTime<Utc>) -> Option<i32> {
        let not_after = self.domain_not_after?;
        matches!(self.config, CheckConfig::DomainExpiry { .. }).then(|| whole_days(not_after, now))
    }
}

/// Whole days from `now` to `end`, rounded down (negative once `end` has passed), held to
/// the range of `i32`.
#[must_use]
pub(in crate::checks) fn whole_days(end: DateTime<Utc>, now: DateTime<Utc>) -> i32 {
    let days = (end - now).num_seconds().div_euclid(86_400);
    i32::try_from(days).unwrap_or(if days < 0 { i32::MIN } else { i32::MAX })
}

/// One stored result of a check (CHK-003), as the heartbeat list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heartbeat {
    pub at: DateTime<Utc>,
    pub status: Status,
    pub response_time_ms: Option<u32>,
    pub message: Option<String>,
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
    /// Every type except HTTP, SNMP and PUSH.
    Config(CheckConfig),
    /// The HTTP check with its secrets in hand; they are sealed apart from `config`.
    Http(HttpInput),
    /// The SNMP check with its secrets in hand; they are sealed apart from `config`.
    Snmp(SnmpInput),
    /// A PUSH check with a new token.
    Push(PushToken),
}

impl Definition {
    #[must_use]
    pub const fn kind(&self) -> CheckType {
        match self {
            Self::Config(config) => config.kind(),
            Self::Http(_) => CheckType::Http,
            Self::Snmp(_) => CheckType::Snmp,
            Self::Push(_) => CheckType::Push,
        }
    }
}

/// What a new check starts with (D-26): the settings the hub's default check takes and the
/// client pre-fills in the check form. Bounds as `checks.checks`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckDefaults {
    pub interval_seconds: u32,
    pub retries: u8,
    pub retry_interval_seconds: u32,
    pub degraded_threshold_ms: u32,
}

impl CheckDefaults {
    /// The settings of a new check: these defaults, not inverted.
    #[must_use]
    pub const fn settings(self) -> CheckSettings {
        CheckSettings {
            interval_seconds: self.interval_seconds,
            retries: self.retries,
            retry_interval_seconds: self.retry_interval_seconds,
            degraded_threshold_ms: self.degraded_threshold_ms,
            invert: false,
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
/// An operator update never changes the type of a check; only the hub's method repair does
/// (CHK-013). An HTTP definition replaces the whole HTTP setup, but an empty secret keeps the
/// stored one: an empty value of a secret header keeps the stored value of the header with
/// that name in the same position, an empty basic password keeps the stored password and an
/// empty bearer token keeps the stored token. An SNMP definition keeps an absent or empty
/// community or password only when the version and user name are the stored ones; otherwise
/// it is a field error that asks for it (SEC-005).
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
    /// True when a group's change moves this check in or out of the group. Every other change
    /// is an operator edit and marks the check (CHK-013).
    pub regrouping: bool,
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
    /// Kept on the check only when the probe reported one.
    pub domain_not_after: Option<DateTime<Utc>>,
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
            domain_not_after: probe.domain_not_after,
        }
    }
}

/// One attempt to record with `record_attempt`: the evaluation runs on the check's stored
/// state and settings inside the write, so a beat, pause or change that landed during the
/// probe is not lost.
#[derive(Clone, Copy, Debug)]
pub(in crate::checks) struct Attempt<'a> {
    pub(in crate::checks) result: &'a ProbeResult,
    /// Maintenance covers the check. It only turns a Down beat into a Maintenance beat
    /// (CHK-006); every other status is recorded as evaluated.
    pub(in crate::checks) in_maintenance: bool,
    pub(in crate::checks) at: DateTime<Utc>,
    /// The domain a domain expiry probe asked about. Its expiry date is kept only while the
    /// check still names that domain (D-29).
    pub(in crate::checks) domain: Option<&'a str>,
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

/// The stored HTTP configuration of a check and the secrets that open for it: what one HTTP
/// probe sends, read together so the secrets go only where this configuration points
/// (SEC-005). Like [`HttpSecrets`] it has no `Debug` or `Clone`.
pub struct HttpSetup {
    pub config: HttpConfig,
    /// `None` when the check holds no secrets.
    pub secrets: Option<HttpSecrets>,
}

/// The stored SNMP configuration of a check and the secrets that open for it, read together
/// so the secrets go only to the version and user they were given for (SEC-005). Like
/// [`SnmpSecrets`] it has no `Debug` or `Clone`.
pub struct SnmpSetup {
    pub config: SnmpConfig,
    /// `None` when the check holds no secrets.
    pub secrets: Option<SnmpSecrets>,
}

/// A check whose next probe is due, as the scheduler needs it.
#[derive(Clone, Debug, PartialEq)]
pub struct Due {
    pub id: CheckId,
    pub device: DeviceId,
    /// The group the check belongs to; maintenance on the group covers it (CHK-006).
    pub parent: Option<CheckId>,
    /// `None` when the stored configuration cannot be read; the probe then fails as a bad
    /// configuration instead of stopping the schedule.
    pub config: Option<CheckConfig>,
    pub settings: CheckSettings,
    pub state: CheckState,
}
