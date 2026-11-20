//! The update of a daemon between it and its hub (K8 Section 8, R-139): what the daemon says in
//! each batch, the version the hub tells it, and the bodies of the two update routes.
//!
//! The hub tells only a version (ROL-009). The release record is bytes to this contract: its
//! fields and its signature are the release record's (ROL-003, ROL-004).

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use super::Switch;

/// The most bytes of a release record, before base64 (K8 Section 8.3).
pub const MAX_RECORD_BYTES: usize = 64 * 1024;

/// The most bytes of the text of a signature file (K8 Section 8.3).
pub const MAX_SIGNATURE_BYTES: usize = 1024;

/// The most tries of one version since the operator last asked (ROL-032).
pub const MAX_TRIES: u8 = 2;

/// The `update` member of a `Batch`: what the daemon says about its own update. A daemon that
/// cannot update itself sends none (ROL-044).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchUpdate {
    /// The target of the installed program, as text: `linux-x86_64` or `darwin-universal`
    /// (OD-ROL-8). The hub reads a target it does not know as "no release for this machine".
    pub target: String,
    /// `off` when updates are set off at the machine (ROL-028).
    pub switch: Switch,
    /// The previous version the machine keeps (ROL-017); absent when it keeps none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,
    /// True while the machine runs a job, a build or a job of a runner (ROL-028).
    pub busy: bool,
    /// The result of the newest try; absent when the machine tried none since its installation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<UpdateTry>,
}

/// The result of the newest try of an update.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateTry {
    pub version: String,
    pub result: TryResult,
    /// Present exactly for `rolled_back` and `refused`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<TryCause>,
    /// The tries of that version since the operator last asked: 1 or 2 (ROL-032).
    pub tries: u8,
}

/// How the newest try ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TryResult {
    Installed,
    RolledBack,
    Refused,
}

/// Why a try did not install: a fixed code, never text from the machine (K8 Section 8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TryCause {
    /// The signature of the record is not good (ROL-010).
    RecordSignature,
    /// The record names a different version, target or program (ROL-010, ROL-013).
    RecordMismatch,
    /// The size or the SHA-256 of the file is not that of the record (ROL-010).
    FileMismatch,
    /// The version is lower than the installed one and is not the previous one (ROL-012).
    LowerVersion,
    /// The record says that the release cannot replace the installed version (ROL-041).
    NotReplaceable,
    /// The record or the file did not arrive whole in 3 tries.
    FetchFailed,
    /// The machine cannot keep the new file and the previous version.
    NoSpace,
    /// The new version failed to start 3 times (ROL-030).
    StartFailed,
    /// 3 minutes passed with no good check (ROL-029, ROL-030).
    NoCheck,
    /// The hub told the previous version, and the machine has it again (ROL-031).
    Asked,
    /// A cause this hub does not know, so a newer daemon can talk to an older hub.
    #[serde(other)]
    Failed,
}

impl TryCause {
    /// Whether this cause can end a try with `result`. `Failed` can end either kind that has a
    /// cause.
    #[must_use]
    pub const fn ends(self, result: TryResult) -> bool {
        match self {
            Self::Failed => !matches!(result, TryResult::Installed),
            Self::StartFailed | Self::NoCheck | Self::Asked => {
                matches!(result, TryResult::RolledBack)
            }
            _ => matches!(result, TryResult::Refused),
        }
    }
}

/// The longest target, in bytes.
const MAX_TARGET_BYTES: usize = 64;

/// A target: 1 to 64 bytes of ASCII letters, digits, `-` and `_` (K8 Section 8.1). A target of
/// that form that the hub does not know is "no release for this machine", not a fault.
fn is_target(text: &str) -> bool {
    (1..=MAX_TARGET_BYTES).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

impl BatchUpdate {
    /// Whether this member keeps the rules of K8 Section 8.1, with `is_version` the rule of
    /// `daemonVersion`; a member that breaks them makes the batch `MALFORMED`.
    pub(super) fn fits(&self, is_version: impl Fn(&str) -> bool) -> bool {
        is_target(&self.target)
            && self.previous.as_deref().is_none_or(&is_version)
            && self.last.as_ref().is_none_or(|last| {
                is_version(&last.version)
                    && (1..=MAX_TRIES).contains(&last.tries)
                    && match (last.result, last.cause) {
                        (TryResult::Installed, None) => true,
                        (result, Some(cause)) => cause.ends(result),
                        (_, None) => false,
                    }
            })
    }
}

/// The `update` member of the hub's `Accepted` answer: the version to have (ROL-009).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInstruction {
    pub version: String,
    /// True only after the operator pressed "Try again" (ROL-032).
    pub again: bool,
}

/// `POST /daemon/v1/update/record` and `POST /daemon/v1/update/file`: the version and the target
/// whose record or file the daemon asks for. It names no file (K8 Section 8.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub version: String,
    pub target: String,
}

/// The answer of `POST /daemon/v1/update/record`: the bytes of the record as standard base64 and
/// the text of its signature file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseRecord {
    pub record: String,
    pub signature: String,
}

impl ReleaseRecord {
    /// The body for `record` and `signature`; `None` when either is over its limit.
    #[must_use]
    pub fn new(record: &[u8], signature: &str) -> Option<Self> {
        (record.len() <= MAX_RECORD_BYTES && signature.len() <= MAX_SIGNATURE_BYTES).then(|| Self {
            record: STANDARD.encode(record),
            signature: signature.to_owned(),
        })
    }

    /// The bytes of the record and the text of the signature; `None` when the record is not
    /// base64 or either is over its limit.
    #[must_use]
    pub fn parts(&self) -> Option<(Vec<u8>, &str)> {
        if self.record.len() > base64::encoded_len(MAX_RECORD_BYTES, true)?
            || self.signature.len() > MAX_SIGNATURE_BYTES
        {
            return None;
        }
        let bytes = STANDARD.decode(&self.record).ok()?;
        (bytes.len() <= MAX_RECORD_BYTES).then_some((bytes, self.signature.as_str()))
    }
}
