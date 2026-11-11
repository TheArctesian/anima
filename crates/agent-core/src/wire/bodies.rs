//! The enrolment and renewal bodies (K8 Sections 3 and 5). The only serde paths for an
//! [`EnrolmentCode`] or an [`AgentToken`] are the fields here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AgentToken, EnrolmentCode, secrets};

/// `POST /agent/v1/enrol`: the code the operator made with `enrolAgent` (K8 Section 3).
#[derive(Serialize, Deserialize)]
pub struct EnrolRequest {
    #[serde(
        serialize_with = "secrets::write_code",
        deserialize_with = "secrets::read_code"
    )]
    pub code: EnrolmentCode,
}

/// The most characters a device name has (K8 Section 3, DEV-010): the hub's device rule takes 1
/// to this many, so the agent keeps every name the hub can hold.
pub const DEVICE_NAME_CHARS: usize = 64;

/// Whether `name` is a device name the agent keeps and prints: 1 to [`DEVICE_NAME_CHARS`]
/// characters, no control character. The hub's device rule has the same length and refuses
/// more characters than this, so every name the hub holds passes (MET-012, K8 Section 3).
#[must_use]
pub fn is_device_name(name: &str) -> bool {
    (1..=DEVICE_NAME_CHARS).contains(&name.chars().count()) && !name.chars().any(char::is_control)
}

/// The answer to a good enrolment: the device the agent now speaks for and its token.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enrolled {
    pub device: Uuid,
    pub device_name: String,
    #[serde(
        serialize_with = "secrets::write_token",
        deserialize_with = "secrets::read_token"
    )]
    pub token: AgentToken,
    #[serde(with = "super::times")]
    pub expires_at: DateTime<Utc>,
}

/// `POST /agent/v1/renew`: the body is `{}`; the token is the bearer (K8 Section 5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenewRequest {}

/// The answer to a renewal: the next token, which replaces the current one on its first use.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Renewed {
    #[serde(
        serialize_with = "secrets::write_token",
        deserialize_with = "secrets::read_token"
    )]
    pub token: AgentToken,
    #[serde(with = "super::times")]
    pub expires_at: DateTime<Utc>,
}
