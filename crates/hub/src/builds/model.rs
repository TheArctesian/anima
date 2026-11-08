//! Domain types of the build queue: releases, build jobs, builders and log lines (spec Section
//! 3.10).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::{fmt, net::IpAddr};

use async_graphql::Enum;
use chrono::{DateTime, Utc};

use super::version::Version;
use crate::ids::{BuildJobId, BuilderId, ReleaseId};

/// What a release is built for (BLD-001, BLD-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enum)]
#[graphql(name = "BuildTarget")]
pub enum Target {
    /// An AppImage, built on Linux on x86_64.
    #[graphql(name = "LINUX_X86_64")]
    LinuxX86_64,
    /// An app and a DMG, built on macOS on aarch64.
    #[graphql(name = "DARWIN_UNIVERSAL")]
    DarwinUniversal,
}

impl Target {
    /// Every target, in display order.
    pub const ALL: [Self; 2] = [Self::LinuxX86_64, Self::DarwinUniversal];

    /// The text stored in `jobs.target`, which is also the folder name under
    /// `releases.anima.ing`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxX86_64 => "linux-x86_64",
            Self::DarwinUniversal => "darwin-universal",
        }
    }

    pub(super) fn from_stored(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|target| target.as_str() == text)
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a build job is (BLD-010). The diagram of Section 3.10.1 says which changes are
/// permitted; `queue` holds that rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "BuildJobState")]
pub enum JobState {
    Queued,
    Claimed,
    Building,
    Uploading,
    Published,
    Failed,
    Cancelled,
}

impl JobState {
    const ALL: [Self; 7] = [
        Self::Queued,
        Self::Claimed,
        Self::Building,
        Self::Uploading,
        Self::Published,
        Self::Failed,
        Self::Cancelled,
    ];

    /// The text stored in `jobs.state`.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::Building => "building",
            Self::Uploading => "uploading",
            Self::Published => "published",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(super) fn from_stored(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.as_str() == text)
    }

    /// Whether a builder holds a lease on a job in this state (BLD-005).
    #[must_use]
    pub const fn is_leased(self) -> bool {
        matches!(self, Self::Claimed | Self::Building | Self::Uploading)
    }
}

/// A builder's operating system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "BuilderOs")]
pub enum Os {
    Linux,
    Macos,
}

/// A builder's processor architecture, as Tauri names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "BuilderArch")]
pub enum Arch {
    #[graphql(name = "X86_64")]
    X86_64,
    #[graphql(name = "AARCH64")]
    Aarch64,
}

/// What a builder runs on; it decides which targets it may claim (BLD-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub(super) const fn os_str(self) -> &'static str {
        match self.os {
            Os::Linux => "linux",
            Os::Macos => "macos",
        }
    }

    pub(super) const fn arch_str(self) -> &'static str {
        match self.arch {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
        }
    }

    pub(super) fn from_stored(os: &str, arch: &str) -> Option<Self> {
        let os = match os {
            "linux" => Os::Linux,
            "macos" => Os::Macos,
            _ => return None,
        };
        let arch = match arch {
            "x86_64" => Arch::X86_64,
            "aarch64" => Arch::Aarch64,
            _ => return None,
        };
        Some(Self { os, arch })
    }
}

/// A commit of `main` as 40 lower-case hex digits. Only `release` makes one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit(pub(super) String);

impl Commit {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// When a job last entered each state, and when it last changed at all (BLD-080).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateTimes {
    pub queued: DateTime<Utc>,
    pub claimed: Option<DateTime<Utc>>,
    pub building: Option<DateTime<Utc>>,
    pub uploading: Option<DateTime<Utc>>,
    pub published: Option<DateTime<Utc>>,
    pub failed: Option<DateTime<Utc>>,
    pub cancelled: Option<DateTime<Utc>>,
    pub changed: DateTime<Utc>,
}

/// The build of one release for one target (Section 3.10.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildJob {
    pub id: BuildJobId,
    pub release: ReleaseId,
    pub target: Target,
    pub version: Version,
    pub commit: Commit,
    pub state: JobState,
    pub failure_reason: Option<String>,
    /// Leases that ended without a report (BLD-006).
    pub attempts: u8,
    pub builder: Option<BuilderId>,
    /// The name of the builder that last claimed it, kept after the builder is revoked.
    pub builder_name: Option<String>,
    pub lease_ends_at: Option<DateTime<Utc>>,
    /// The sequence number of the last log line written.
    pub log_sequence: i64,
    pub created_at: DateTime<Utc>,
    pub times: StateTimes,
}

/// A version and commit with one job for each selected target (BLD-001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub id: ReleaseId,
    pub version: Version,
    pub commit: Commit,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub jobs: Vec<BuildJob>,
}

/// A computer the operator enrolled to build releases (BLD-021).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Builder {
    pub id: BuilderId,
    pub name: String,
    pub platform: Platform,
    /// The VPN address it enrolled from; it must call from there (BLD-024).
    pub client: IpAddr,
    pub enrolled_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

/// One line of a job's log, after redaction (BLD-048, BLD-049).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    pub sequence: i64,
    pub text: String,
}

/// A file a builder reported uploaded (BLD-063).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadedFile {
    pub name: String,
    /// 64 lower-case hex digits.
    pub sha256: String,
    pub size: i64,
}

/// A change of a job for the live view: its state now and the log lines added (BLD-081).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobUpdate {
    pub job: BuildJob,
    pub lines: Vec<LogLine>,
}

/// An input field the rules refuse. The message never holds a secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    /// The GraphQL argument or input field in camelCase.
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

/// What a lookup could not find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Job(BuildJobId),
    Builder(BuilderId),
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Job(id) => write!(f, "build job {id}"),
            Self::Builder(id) => write!(f, "builder {id}"),
        }
    }
}
