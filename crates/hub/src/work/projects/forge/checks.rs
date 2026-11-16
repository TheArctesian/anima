//! The checks of one commit: its check runs and its combined status, read live and counted
//! together by the provider (PRJ-032; decisions item 8).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Context, Enum, Result, SimpleObject};
use provider_github::live::{
    checks::{self as github, Counts, StatusState},
    runs::RunStatus,
};
use source_exchange::{Body, Response};

use super::{Target, parsed, read};
use crate::engine::sources::Keep;

/// Checks with any not finished: read again after 30 seconds; else kept 10 minutes.
const CHECKS: Keep = Keep::for_seconds(30);
const SETTLED_SECONDS: i64 = 600;

// The overall state of a commit's checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ChecksState")]
pub enum ChecksState {
    None,
    Pending,
    Passed,
    Failed,
}

/// The check runs and commit statuses of one commit, counted together.
#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "CommitChecks")]
pub struct CommitChecksView {
    sha: String,
    /// FAILED when any failed; else PENDING when any is not finished; else PASSED; NONE when the commit has none.
    state: ChecksState,
    passed: i32,
    failed: i32,
    pending: i32,
    /// Skipped, neutral or cancelled.
    other: i32,
    /// Up to 10 names of failed checks, each at most 100 characters. Text from GitHub.
    failing: Vec<String>,
    /// True: the commit has more than 100 check runs or statuses; the counts cover the first 100 of each.
    partial: bool,
}

fn view(sha: String, counts: Counts) -> CommitChecksView {
    let int = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    CommitChecksView {
        sha,
        state: match counts.state {
            github::ChecksState::None => ChecksState::None,
            github::ChecksState::Pending => ChecksState::Pending,
            github::ChecksState::Passed => ChecksState::Passed,
            github::ChecksState::Failed => ChecksState::Failed,
        },
        passed: int(counts.passed),
        failed: int(counts.failed),
        pending: int(counts.pending),
        other: int(counts.other),
        failing: counts.failing,
        partial: counts.partial,
    }
}

fn answer(body: &[u8]) -> Response<'_> {
    Response {
        status: 200,
        body: Body::Read(body),
    }
}

/// The body holds check runs and every one has finished. A commit with none yet may get some
/// soon, so it is not settled.
fn runs_settled(body: &[u8]) -> bool {
    github::check_runs_parse(&answer(body)).is_ok_and(|runs| {
        !runs.runs.is_empty()
            && runs
                .runs
                .iter()
                .all(|run| run.status == RunStatus::Completed)
    })
}

/// The body holds statuses and none is pending.
fn statuses_settled(body: &[u8]) -> bool {
    github::status_parse(&answer(body)).is_ok_and(|combined| {
        !combined.statuses.is_empty()
            && combined
                .statuses
                .iter()
                .all(|status| status.state != StatusState::Pending)
    })
}

/// The checks of `sha` in `target`: two GitHub requests, each kept 30 seconds, or 10 minutes
/// once nothing in it is pending. `None` when GitHub has no such commit.
///
/// # Errors
/// The K7 code and fixed words of a failed read.
pub async fn commit_checks(
    ctx: &Context<'_>,
    target: &Target,
    sha: &str,
    fresh: bool,
) -> Result<Option<CommitChecksView>> {
    let runs_keep = CHECKS
        .longer_when(SETTLED_SECONDS, runs_settled)
        .fresh(fresh);
    let request = github::check_runs_request(&target.name, sha, 1);
    let Some(runs) = read(ctx, target, request, runs_keep).await? else {
        return Ok(None);
    };
    let runs = parsed(&runs, github::check_runs_parse)?;
    let statuses_keep = CHECKS
        .longer_when(SETTLED_SECONDS, statuses_settled)
        .fresh(fresh);
    let request = github::status_request(&target.name, sha, 1);
    let Some(statuses) = read(ctx, target, request, statuses_keep).await? else {
        return Ok(None);
    };
    let statuses = parsed(&statuses, github::status_parse)?;
    Ok(Some(view(sha.to_owned(), github::count(&runs, &statuses))))
}
