//! The hub's refusals and what the daemon does with each (K8 Section 6).

use serde::{Deserialize, Serialize};

/// Why the hub refused a request: the `error` of a refusal body (K8 Section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefusalCode {
    /// The body is not a valid `Batch` or `{}`.
    Malformed,
    /// The token is missing, malformed, unknown, expired or revoked.
    TokenRefused,
    /// The enrolment code is unknown, expired or used, or the enrol body is malformed.
    EnrolmentRefused,
    /// The batch names a device other than the token's.
    WrongDevice,
    /// The body exceeds its limit.
    TooLarge,
    /// A point's `at` is more than 1 hour before or 60 seconds after the hub's clock.
    OutOfRange,
    /// The hub cannot store now.
    Unavailable,
    /// The key does not exist, has no active version, or the device is no consumer of it: one
    /// answer for the three, so a server cannot learn which keys exist (K-E5 Section 2.1).
    NoKey,
    /// The device does not hold the job, the attempt is not the current one, the job has
    /// ended, or the change of state is not permitted (K10 Section 7).
    NotHolder,
    /// The operator set the device off for work in the hub, or the hub gives it no work for
    /// 1 hour (K10 Section 11).
    WorkOff,
    /// The secret is not given; the answer does not say which test failed (K10 Section 8).
    SecretRefused,
}

/// A refusal body, `{ "error": "<CODE>" }`, and nothing else: it never echoes the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    pub error: RefusalCode,
}

impl RefusalCode {
    /// The HTTP status the hub answers with.
    #[must_use]
    pub const fn status(self) -> u16 {
        match self {
            Self::Malformed => 400,
            Self::TokenRefused | Self::EnrolmentRefused => 401,
            Self::WrongDevice | Self::WorkOff => 403,
            Self::NotHolder | Self::SecretRefused => 409,
            Self::TooLarge => 413,
            Self::OutOfRange => 422,
            Self::NoKey => 404,
            Self::Unavailable => 503,
        }
    }

    /// The code as the body spells it, for the event log.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Malformed => "MALFORMED",
            Self::TokenRefused => "TOKEN_REFUSED",
            Self::EnrolmentRefused => "ENROLMENT_REFUSED",
            Self::WrongDevice => "WRONG_DEVICE",
            Self::TooLarge => "TOO_LARGE",
            Self::OutOfRange => "OUT_OF_RANGE",
            Self::Unavailable => "UNAVAILABLE",
            Self::NoKey => "NO_KEY",
            Self::NotHolder => "NOT_HOLDER",
            Self::WorkOff => "WORK_OFF",
            Self::SecretRefused => "SECRET_REFUSED",
        }
    }

    /// Whether the daemon drops a batch the hub refused with this code, rather than keep its
    /// points to send again: a batch that is malformed, too large or out of range never
    /// becomes acceptable (K8 Section 6).
    #[must_use]
    pub const fn drops_batch(self) -> bool {
        matches!(self, Self::Malformed | Self::TooLarge | Self::OutOfRange)
    }
}

impl From<RefusalCode> for Refusal {
    fn from(error: RefusalCode) -> Self {
        Self { error }
    }
}
