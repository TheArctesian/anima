//! The bodies of the work route between `anima-daemon` and the hub
//! (`contracts/work-route.md`, K10): the claim of a job, the job, the report with its log part
//! and its result, and the request for a secret of one job.
//!
//! No body holds a secret but the answer to a secret request, which this file does not type:
//! a job never holds a command, an argument, a path, an address, an image name or an
//! environment variable (K10 Section 4). The contract calls a job an "item of work".

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::times;

/// The most bytes of the log part of one report (K10 Section 7).
pub const MAX_LOG_PART_BYTES: usize = 64 * 1024;

/// A work type of the fixed list (K10 Section 6). A type that this program does not know reads
/// as [`WorkType::Unknown`]: the hub gives no such job, and the daemon refuses one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkType {
    Build,
    Runner,
    /// The model work types (`contracts/compute/model-work.md` Section 2; waits for
    /// R-NEW-modelwork). Each has the kind [`WorkKind::Model`].
    ModelReport,
    RuntimeInstall,
    ModelPull,
    CopyDelete,
    ServerStart,
    ServerStop,
    ModelLoad,
    ModelUnload,
    ModelMeasure,
    ModelDrain,
    #[serde(other)]
    Unknown,
}

/// The ten model work types, in the order of the table of the model work contract.
pub const MODEL_TYPES: [WorkType; 10] = [
    WorkType::ModelReport,
    WorkType::RuntimeInstall,
    WorkType::ModelPull,
    WorkType::CopyDelete,
    WorkType::ServerStart,
    WorkType::ServerStop,
    WorkType::ModelLoad,
    WorkType::ModelUnload,
    WorkType::ModelMeasure,
    WorkType::ModelDrain,
];

impl WorkType {
    /// The type as the bodies spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Runner => "runner",
            Self::ModelReport => "model_report",
            Self::RuntimeInstall => "runtime_install",
            Self::ModelPull => "model_pull",
            Self::CopyDelete => "copy_delete",
            Self::ServerStart => "server_start",
            Self::ServerStop => "server_stop",
            Self::ModelLoad => "model_load",
            Self::ModelUnload => "model_unload",
            Self::ModelMeasure => "model_measure",
            Self::ModelDrain => "model_drain",
            Self::Unknown => "unknown",
        }
    }

    /// The type that `text` spells, when it is of the fixed list.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Build, Self::Runner]
            .into_iter()
            .chain(MODEL_TYPES)
            .find(|kind| kind.as_str() == text)
    }

    /// Whether the type is one of the model work.
    #[must_use]
    pub fn is_model(self) -> bool {
        MODEL_TYPES.contains(&self)
    }
}

/// How a job is isolated on its machine (K10 Section 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    Container,
    Native,
    /// The daemon does the steps itself; no job code runs (model work, Section 1).
    Model,
    #[serde(other)]
    Unknown,
}

/// The switch at the machine (USE-004). The hub cannot set it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Switch {
    On,
    Off,
}

/// The operating system of a machine that claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkOs {
    Linux,
    Macos,
}

/// The processor architecture of a machine that claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkArchitecture {
    #[serde(rename = "x86_64")]
    X86_64,
    Aarch64,
}

/// What a machine has free for a job now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Free {
    pub slots: u32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
}

/// One job that a machine runs now, by its attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub id: Uuid,
    pub attempt: u32,
}

/// `POST /work/v1/claim` (K10 Section 4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub device: Uuid,
    pub switch: Switch,
    pub types: Vec<WorkType>,
    pub kinds: Vec<WorkKind>,
    pub os: WorkOs,
    pub architecture: WorkArchitecture,
    pub free: Free,
    #[serde(default)]
    pub holds: Vec<Held>,
    #[serde(default)]
    pub unclean: bool,
}

/// The end of a lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    #[serde(with = "times")]
    pub until: DateTime<Utc>,
}

/// The limits of one job. The machine uses the lower of each and its own limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub seconds: u64,
    pub idle_seconds: u64,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub orphan_seconds: u64,
}

/// The repository of a job, by its forge and its two names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRepository {
    pub forge: String,
    pub owner: String,
    pub name: String,
}

/// The answer to a claim that gets a job: the `Item` of K10 Section 4.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkJob {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub work_type: WorkType,
    pub kind: WorkKind,
    pub attempt: u32,
    pub lease: Lease,
    pub limits: Limits,
    pub project: Option<Uuid>,
    pub repository: WorkRepository,
    /// The purposes of the secrets that the job needs. Never a value.
    pub secrets: Vec<String>,
    /// The object of the work type (K10 Section 6).
    pub parameters: serde_json::Value,
}

/// The state of an attempt, as a machine reports it (K10 Section 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkState {
    Claimed,
    Preparing,
    Running,
    Finishing,
    Done,
    Failed,
    Stopped,
    Refused,
}

impl WorkState {
    /// Whether an attempt in this state has ended.
    #[must_use]
    pub const fn is_final(self) -> bool {
        matches!(
            self,
            Self::Done | Self::Failed | Self::Stopped | Self::Refused
        )
    }

    /// The state as the bodies spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claimed => "claimed",
            Self::Preparing => "preparing",
            Self::Running => "running",
            Self::Finishing => "finishing",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
            Self::Refused => "refused",
        }
    }
}

/// The next part of the log of a job: `from` is the count of bytes before it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogPart {
    pub from: u64,
    pub text: String,
}

/// What a machine deleted at the end of a job. Each member is `deleted`, `failed` or `none`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cleanup {
    pub isolation: String,
    pub checkout: String,
    pub secrets: String,
    pub keychain: String,
}

/// The result of an attempt that ended: fixed codes and numbers, never text from the job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkResult {
    pub outcome: String,
    pub cause: Option<String>,
    pub seconds: u64,
    pub cleanup: Cleanup,
}

/// `POST /work/v1/report` (K10 Section 7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkReport {
    pub id: Uuid,
    pub attempt: u32,
    pub state: WorkState,
    #[serde(default)]
    pub log: Option<LogPart>,
    #[serde(default)]
    pub result: Option<WorkResult>,
}

/// Why the hub tells a machine to stop a job (K10 Section 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Operator,
    RunnersOff,
    Update,
    Superseded,
    Expired,
}

impl StopReason {
    /// The reason as the bodies spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::RunnersOff => "runners_off",
            Self::Update => "update",
            Self::Superseded => "superseded",
            Self::Expired => "expired",
        }
    }
}

/// The `stop` member of [`Reported`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopOrder {
    pub reason: StopReason,
}

/// The answer to a report: the new lease, a stop when there is one, and the end of the log that
/// the hub holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reported {
    pub lease: Lease,
    pub stop: Option<StopOrder>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_end: Option<u64>,
}

/// `POST /work/v1/secret` (K10 Section 8). The answer holds a value and has no type here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRequest {
    pub id: Uuid,
    pub attempt: u32,
    pub purpose: String,
}
