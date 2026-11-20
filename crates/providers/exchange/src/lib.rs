//! What a source provider and the engine say to each other (SRC-001, SRC-006, REPO-E-002).
//!
//! Pure types, no I/O and no hub imports. A provider crate depends on this crate only, so it
//! cannot reach the network, the database or a secret: it names the request it wants and
//! reads the reply the engine hands back. A write is a named [`Change`] of checked parts
//! (OD-E14), never a free-form request.

mod change;
mod declaration;
mod exchange;
mod parts;
mod registration;

pub use change::{
    Change, ChangePath, ChangeReply, Changes, Fixed, Limit, Receipt, Refusal, Rendered, Settled,
    Verb,
};
pub use declaration::{
    AccountKey, Acquisition, AppKey, Declaration, MIN_POLL_INTERVAL_SECONDS, Minted, Minting,
};
pub use exchange::{Body, Method, Normalised, Problem, Request, RequestError, Response};
pub use parts::{
    BranchName, CloseReason, CommitSha, InputName, InputValue, Inputs, IssueState, Label, Labels,
    MergeMethod, Number, PartError, RepositoryName, RunId, StatusContext, StatusDescription,
    StatusState, TargetUrl, Text, Title, WorkflowId,
};
pub use registration::{Converted, Installation, ManifestFacts, Owner, Registration, Webhook};

#[cfg(test)]
mod tests;
