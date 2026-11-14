//! What came of each pull request of a bulk merge (K1 `PullRequestMerge`; PRJ-033), and the
//! deadline of the whole request (K-E1 Section 13.3). Pure: the time is a parameter.
//!
//! A message is always the hub's fixed words, never a reply of GitHub or text from the pull
//! request (LOG-007).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, Error as GraphQlError, ID, SimpleObject, Value};
use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::{Receipt, Refusal, Settled};

use super::{input::Entry, merge};
use crate::{
    api_errors::ErrorCode,
    work::projects::forge::{Subject, failure, int},
};

/// How long a bulk merge may take, from its start to the start of its last merge.
pub const DEADLINE: TimeDelta = TimeDelta::seconds(90);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum MergeOutcome {
    Merged,
    /// It was merged before this request. Nothing was changed.
    AlreadyMerged,
    HeadMoved,
    /// Conflict, a failed or missing check, a draft, or closed.
    NotMergeable,
    /// GitHub had not computed whether it can merge. Try again.
    NotReady,
    /// From a fork: merge it alone, with forkConfirmed (PRO-013).
    ForkRefused,
    /// The repository does not allow this merge method.
    MethodRefused,
    NotFound,
    /// GitHub refused or could not be reached. Nothing was changed.
    Failed,
    /// The request was sent and no answer came. It may be merged: read the list again.
    Unknown,
    /// The hub stopped before it reached this one (90 seconds for the whole request).
    NotAttempted,
}

#[derive(Clone, Debug, PartialEq, Eq, SimpleObject)]
#[graphql(name = "PullRequestMerge")]
pub struct PullRequestMerge {
    pub repository: ID,
    pub number: i32,
    pub outcome: MergeOutcome,
    /// Fixed words. Never a provider reply (LOG-007).
    pub message: String,
    /// Set for MERGED.
    pub merge_sha: Option<String>,
}

/// Whether a bulk merge that started at `started` may not start another merge at `now`.
#[must_use]
pub fn past(started: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - started >= DEADLINE
}

fn result(entry: &Entry, outcome: MergeOutcome, message: impl Into<String>) -> PullRequestMerge {
    PullRequestMerge {
        repository: entry.repository.clone(),
        number: int(entry.seen.number.get()),
        outcome,
        message: message.into(),
        merge_sha: None,
    }
}

/// The hub stopped before it reached `entry`.
#[must_use]
pub fn not_attempted(entry: &Entry) -> PullRequestMerge {
    result(
        entry,
        MergeOutcome::NotAttempted,
        "The hub stopped before it reached this one: 90 seconds passed. Nothing was changed.",
    )
}

/// `entry` was merged before this request.
#[must_use]
pub fn already(entry: &Entry) -> PullRequestMerge {
    result(
        entry,
        MergeOutcome::AlreadyMerged,
        "It was merged before this request. Nothing was changed.",
    )
}

/// The hub refused `entry` before sending anything.
#[must_use]
pub fn refused(entry: &Entry, refusal: merge::Refusal) -> PullRequestMerge {
    result(entry, refusal.outcome(), refusal.words())
}

/// GitHub has no such pull request, or the App may not read it.
#[must_use]
pub fn missing(entry: &Entry) -> PullRequestMerge {
    result(
        entry,
        MergeOutcome::NotFound,
        "GitHub has no such pull request, or the App may not read it.",
    )
}

/// What came of the merge of `entry` that was sent.
#[must_use]
pub fn sent(entry: &Entry, settled: Settled) -> PullRequestMerge {
    let outcome = match &settled {
        Settled::Done(_) => MergeOutcome::Merged,
        Settled::Refused(Refusal::HeadMoved) => MergeOutcome::HeadMoved,
        Settled::Refused(Refusal::NotMergeable) => MergeOutcome::NotMergeable,
        Settled::Refused(Refusal::NotFound) => MergeOutcome::NotFound,
        Settled::Unknown => MergeOutcome::Unknown,
        Settled::Refused(_) | Settled::Limited { .. } => MergeOutcome::Failed,
    };
    let merge_sha = match &settled {
        Settled::Done(Receipt::Merged { sha }) => Some(sha.as_str().to_owned()),
        _ => None,
    };
    let message = match failure::settled(settled, Subject::PullRequest) {
        Ok(_) => "Merged.".to_owned(),
        Err(error) => error.message,
    };
    PullRequestMerge {
        merge_sha,
        ..result(entry, outcome, message)
    }
}

/// `entry` failed before anything was sent, with the hub's fixed words of `error`: `NOT_FOUND`
/// for a repository or pull request the hub or GitHub does not have, `FAILED` otherwise.
#[must_use]
pub fn failed(entry: &Entry, error: &GraphQlError) -> PullRequestMerge {
    let outcome = if code(error) == Some(ErrorCode::NotFound.as_str()) {
        MergeOutcome::NotFound
    } else {
        MergeOutcome::Failed
    };
    result(entry, outcome, error.message.clone())
}

/// Whether `error` is the hub's own failure (its database): the bulk merge stops and says so,
/// as one merge would (K-E1 Section 13.6).
#[must_use]
pub fn is_internal(error: &GraphQlError) -> bool {
    code(error) == Some(ErrorCode::Internal.as_str())
}

fn code(error: &GraphQlError) -> Option<&str> {
    match error.extensions.as_ref()?.get("code")? {
        Value::String(code) => Some(code.as_str()),
        _ => None,
    }
}
