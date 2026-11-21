//! The K8 bodies between `anima-daemon` and the hub (`contracts/daemon-push.md`): enrolment,
//! push, renewal, update and refusals, with the two secrets and the hub's pure check of a batch
//! (MET-012, MET-013, MET-014, MET-017).
//!
//! Member names are camelCase, unknown members are ignored, and times are RFC 3339 UTC with
//! `Z` and whole seconds (K8 Section 1).

mod agent_usage;
mod batch;
mod batteries;
mod bodies;
mod check;
mod inventory;
mod keys;
mod refusal;
mod secrets;
#[cfg(test)]
mod tests;
mod times;
mod update;
mod work;

pub use agent_usage::{BatchAgentUsage, BatchModel, BatchSessionDay};
pub use batch::{
    Announcements, Batch, BatchDrive, BatchGpu, BatchPoint, BatchSeries, DriveKind, GpuVendor,
};
pub use batteries::{BatchBattery, BatteryState};
pub use bodies::{
    DEVICE_NAME_CHARS, EnrolRequest, Enrolled, RenewRequest, Renewed, is_device_name,
};
pub use check::Accepted;
pub use inventory::{BatchInventory, BatchTool};
pub use keys::{
    BatchKeys, FetchedKey, KeyFetch, KeyValue, KeyVersion, KeysCurrent, MAX_KEYS, MAX_VALUE_BYTES,
    PushAccepted, is_key_name, versions_fit,
};
pub use refusal::{Refusal, RefusalCode};
pub use secrets::{CODE_BYTES, CodeHash, DaemonToken, EnrolmentCode, TOKEN_BYTES, TokenHash};
pub use update::{
    BatchUpdate, MAX_RECORD_BYTES, MAX_SIGNATURE_BYTES, MAX_TRIES, ReleaseRecord, TryCause,
    TryResult, UpdateInstruction, UpdateRequest, UpdateTry,
};
pub use work::{
    Claim, Cleanup, Free, Held, Lease, Limits, LogPart, MAX_LOG_PART_BYTES, MODEL_TYPES, Reported,
    SecretRequest, StopOrder, StopReason, Switch, WorkArchitecture, WorkJob, WorkKind, WorkOs,
    WorkReport, WorkRepository, WorkResult, WorkState, WorkType,
};
