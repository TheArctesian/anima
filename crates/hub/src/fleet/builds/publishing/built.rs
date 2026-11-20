//! One release of one target that the jobs of the release workflow built (R-179): the checks
//! the publisher makes for a queued job (BLD-002, BLD-046, BLD-064 to BLD-069), with no queue
//! and no database. `checks` holds what needs only the bytes, `publisher` the bucket, and
//! `command` the `anima-hub release` commands, and `record` the release record of the
//! program files of a target (task U2). A refusal names files and versions only; no
//! key, signature, token or URL reaches an error (LOG-007).

mod checks;
mod command;
mod publisher;
mod record;

use bytes::Bytes;

pub use checks::{CheckedBuilt, check_built, sign_built};
pub use command::{ReleaseError, release_command};
pub use publisher::BuiltPublisher;

use super::transfer::Stop;
use crate::fleet::builds::{model::Target, version::Version};

/// One file a job made, with its bytes.
pub struct BuiltFile {
    pub name: String,
    pub bytes: Bytes,
}

/// The files of one target at one version.
pub struct Built {
    pub target: Target,
    pub version: Version,
    pub files: Vec<BuiltFile>,
}

/// Why a built release is not signed or not published.
#[derive(Debug, thiserror::Error)]
pub enum BuiltError {
    /// A check failed; nothing was written after it, and no feed.
    #[error("{0}")]
    Refused(String),
    /// The bucket or the public address did not answer; the same command can run again.
    #[error("the release bucket or the public address cannot be reached ({0})")]
    Unavailable(&'static str),
}

impl From<Stop> for BuiltError {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::Refused(reason) => Self::Refused(reason),
            Stop::Unavailable(fault) => Self::Unavailable(fault),
            Stop::TakenOver => Self::Unavailable("hold"),
        }
    }
}

fn refused(reason: String) -> BuiltError {
    BuiltError::Refused(reason)
}
