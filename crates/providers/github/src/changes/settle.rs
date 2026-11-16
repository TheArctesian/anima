//! What GitHub's reply to a named change means (OD-E14, design C.6, decisions items 2 and 7).
//!
//! Only the status, the rate limit headers the engine read and, on success, a number or a
//! commit hash from the body count. No text of the reply reaches the outcome (LOG-007).

use chrono::{DateTime, TimeDelta, Utc};
use serde::Deserialize;
use source_exchange::{
    Body, Change, ChangeReply, CommitSha, Limit, Number, Receipt, Refusal, Settled,
};

/// How long to wait after a limit that names no time: "wait for at least one minute"
/// (GitHub's rate limit page).
const LIMIT_WAIT_SECONDS: u64 = 60;

/// The outcome of the reply to `change`.
///
/// A rate limit (429, or 403 with no requests remaining or a `retry-after`) comes first. Then
/// the documented success status is done. Another success status, or a server error, does not
/// say whether GitHub made the change, so it is [`Settled::Unknown`]; every other status is a
/// refusal.
pub(crate) fn settle(change: &Change, reply: &ChangeReply<'_>) -> Settled {
    if let Some(until) = limited(reply) {
        return Settled::Limited { until };
    }
    if succeeded(change, reply.status) {
        return done(change, reply.body);
    }
    match reply.status {
        200..=299 | 500..=599 => Settled::Unknown,
        status => Settled::Refused(refusal(change, status)),
    }
}

/// Whether `status` is the success GitHub documents for `change`. A dispatch is 204 on
/// 2022-11-28, and 200 when GitHub returns the run (2026-03-10, or `return_run_details`).
const fn succeeded(change: &Change, status: u16) -> bool {
    match change {
        Change::CreateIssue { .. }
        | Change::AddIssueComment { .. }
        | Change::RerunWorkflowRun { .. }
        | Change::RerunFailedWorkflowJobs { .. }
        | Change::SetCommitStatus { .. } => status == 201,
        Change::SetIssueOpen { .. } | Change::MergePullRequest { .. } => status == 200,
        Change::CancelWorkflowRun { .. } => status == 202,
        Change::DispatchWorkflow { .. } => matches!(status, 200 | 204),
    }
}

/// The refusal a status that is neither a success nor a rate limit stands for. For a merge,
/// 405, 409 and 422 are GitHub refusing the merge; 409 is the head that moved (decisions
/// item 2).
const fn refusal(change: &Change, status: u16) -> Refusal {
    let merge = matches!(change, Change::MergePullRequest { .. });
    let cancel = matches!(change, Change::CancelWorkflowRun { .. });
    match status {
        401 => Refusal::KeyRefused,
        403 => Refusal::NotPermitted,
        404 => Refusal::NotFound,
        409 if merge => Refusal::HeadMoved,
        405 | 422 if merge => Refusal::NotMergeable,
        409 if cancel => Refusal::Conflict,
        410 => Refusal::Gone,
        422 => Refusal::Invalid,
        _ => Refusal::Unexplained,
    }
}

/// When the reply is a rate limit, the time to wait until (decisions item 7).
fn limited(reply: &ChangeReply<'_>) -> Option<DateTime<Utc>> {
    let limit = &reply.limit;
    let spent = limit.remaining == Some(0) || limit.retry_after_seconds.is_some();
    let is_limit = reply.status == 429 || (reply.status == 403 && spent);
    is_limit.then(|| until(limit, reply.at))
}

/// The later of `retry-after` and, with no requests remaining, the reset time. A minute after
/// `at` when neither names a time after `at`.
fn until(limit: &Limit, at: DateTime<Utc>) -> DateTime<Utc> {
    let retry = limit.retry_after_seconds.map(|seconds| after(at, seconds));
    let reset = limit.reset_at.filter(|_| limit.remaining == Some(0));
    retry
        .into_iter()
        .chain(reset)
        .max()
        .filter(|until| *until > at)
        .unwrap_or_else(|| after(at, LIMIT_WAIT_SECONDS))
}

/// `seconds` after `at`, or the last time there is when that is past it.
fn after(at: DateTime<Utc>, seconds: u64) -> DateTime<Utc> {
    i64::try_from(seconds)
        .ok()
        .and_then(TimeDelta::try_seconds)
        .and_then(|wait| at.checked_add_signed(wait))
        .unwrap_or(DateTime::<Utc>::MAX_UTC)
}

#[derive(Deserialize)]
struct NewIssue {
    number: u64,
}

#[derive(Deserialize)]
struct NewComment {
    id: u64,
}

#[derive(Deserialize)]
struct Merge {
    merged: bool,
    sha: Option<String>,
}

/// The receipt of a change GitHub made. A body that cannot be read, or lacks the number or
/// the commit, still means the change was made: [`Receipt::Made`]. A merge reply that says it
/// did not merge contradicts its status: [`Settled::Unknown`].
fn done(change: &Change, body: Body<'_>) -> Settled {
    let Body::Read(bytes) = body else {
        return Settled::Done(Receipt::Made);
    };
    let receipt = match change {
        Change::CreateIssue { .. } => serde_json::from_slice::<NewIssue>(bytes)
            .ok()
            .and_then(|issue| created(issue.number)),
        Change::AddIssueComment { .. } => serde_json::from_slice::<NewComment>(bytes)
            .ok()
            .and_then(|comment| created(comment.id)),
        Change::MergePullRequest { .. } => match serde_json::from_slice::<Merge>(bytes) {
            Ok(Merge { merged: false, .. }) => return Settled::Unknown,
            Ok(merge) => merge
                .sha
                .and_then(|sha| CommitSha::parse(&sha).ok())
                .map(|sha| Receipt::Merged { sha }),
            Err(_) => None,
        },
        _ => None,
    };
    Settled::Done(receipt.unwrap_or(Receipt::Made))
}

/// A number GitHub gave the new thing, when it is one a part may hold (1 to 2^53).
fn created(number: u64) -> Option<Receipt> {
    Number::new(number).ok().map(|number| Receipt::Created {
        number: number.get(),
    })
}
