//! The K8 bodies between `anima-agent` and the hub (`contracts/agent-push.md`): enrolment,
//! push, renewal and refusals, with the two secrets and the hub's pure check of a batch
//! (MET-012, MET-013, MET-014, MET-017).
//!
//! Member names are camelCase, unknown members are ignored, and times are RFC 3339 UTC with
//! `Z` and whole seconds (K8 Section 1).

mod batch;
mod bodies;
mod check;
mod refusal;
mod secrets;
#[cfg(test)]
mod tests;
mod times;

pub use batch::{Batch, BatchDrive, BatchGpu, BatchPoint, BatchSeries, DriveKind, GpuVendor};
pub use bodies::{
    DEVICE_NAME_CHARS, EnrolRequest, Enrolled, RenewRequest, Renewed, is_device_name,
};
pub use check::Accepted;
pub use refusal::{Refusal, RefusalCode};
pub use secrets::{AgentToken, CODE_BYTES, CodeHash, EnrolmentCode, TOKEN_BYTES, TokenHash};
