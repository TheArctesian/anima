//! The check runs and the combined commit status of a ref, counted together (PRJ-032).

use serde::Deserialize;
use source_exchange::{Problem, Request, Response};

use super::runs::{Conclusion, RunStatus};
use super::{get, read};
use crate::name;
use crate::problem::problem;

/// The most names of failing checks kept, and the most characters of each.
const MOST_FAILING: usize = 10;
const NAME_LENGTH: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRun {
    pub name: String,
    pub status: RunStatus,
    pub conclusion: Option<Conclusion>,
}

/// One page of check runs with GitHub's total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRuns {
    pub total_count: u64,
    pub runs: Vec<CheckRun>,
}

/// A commit status `state`, an open string: anything not named here is `Other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusState {
    Success,
    Failure,
    Error,
    Pending,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitStatus {
    pub context: String,
    pub state: StatusState,
}

/// One page of the combined status. GitHub's own `state` is not kept: it reads `pending` for
/// a commit with no statuses, so [`count`] works from the statuses themselves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombinedStatus {
    pub total_count: u64,
    pub statuses: Vec<CommitStatus>,
}

#[derive(Deserialize)]
struct RawCheckRun {
    name: String,
    status: Option<String>,
    conclusion: Option<String>,
}

#[derive(Deserialize)]
struct RawCheckRuns {
    total_count: u64,
    check_runs: Vec<RawCheckRun>,
}

#[derive(Deserialize)]
struct RawStatus {
    context: String,
    state: String,
}

#[derive(Deserialize)]
struct RawCombined {
    total_count: u64,
    statuses: Vec<RawStatus>,
}

fn ref_path(git_ref: &str, tail: &str) -> Result<String, Problem> {
    if git_ref.is_empty() || name::check_branch(git_ref).is_err() {
        return Err(problem(
            "ref_unusable",
            "checks",
            "the ref cannot be used to read its checks",
        ));
    }
    Ok(format!("/commits/{}{tail}", name::encode_branch(git_ref)))
}

/// `GET /repos/{o}/{r}/commits/{ref}/check-runs`, 100 to a page. `git_ref` is a commit sha or
/// `heads/BRANCH`; one with a dot segment is refused.
///
/// # Errors
/// A [`Problem`] when the repository name or the ref cannot go into a path.
pub fn check_runs_request(repository: &str, git_ref: &str, page: u32) -> Result<Request, Problem> {
    get(
        repository,
        &ref_path(git_ref, "/check-runs")?,
        &[],
        Some(page),
    )
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of check runs.
pub fn check_runs_parse(response: &Response<'_>) -> Result<CheckRuns, Problem> {
    let raw: RawCheckRuns = read(response, "checks")?;
    Ok(CheckRuns {
        total_count: raw.total_count,
        runs: raw
            .check_runs
            .into_iter()
            .map(|run| CheckRun {
                name: run.name,
                status: RunStatus::named(run.status.as_deref()),
                conclusion: run.conclusion.as_deref().map(Conclusion::named),
            })
            .collect(),
    })
}

/// `GET /repos/{o}/{r}/commits/{ref}/status`, 100 to a page.
///
/// # Errors
/// A [`Problem`] when the repository name or the ref cannot go into a path.
pub fn status_request(repository: &str, git_ref: &str, page: u32) -> Result<Request, Problem> {
    get(repository, &ref_path(git_ref, "/status")?, &[], Some(page))
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a combined status.
pub fn status_parse(response: &Response<'_>) -> Result<CombinedStatus, Problem> {
    let raw: RawCombined = read(response, "checks")?;
    let statuses: Vec<CommitStatus> = raw
        .statuses
        .into_iter()
        .map(|status| CommitStatus {
            context: status.context,
            state: match status.state.as_str() {
                "success" => StatusState::Success,
                "failure" => StatusState::Failure,
                "error" => StatusState::Error,
                "pending" => StatusState::Pending,
                _ => StatusState::Other,
            },
        })
        .collect();
    Ok(CombinedStatus {
        total_count: raw.total_count,
        statuses,
    })
}

/// The overall state of a commit's checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChecksState {
    None,
    Pending,
    Passed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counts {
    pub state: ChecksState,
    pub passed: u32,
    pub failed: u32,
    pub pending: u32,
    /// Skipped, neutral, cancelled, stale or a value this crate does not know.
    pub other: u32,
    /// Up to 10 names of failed checks, each cut to 100 characters. Text from GitHub.
    pub failing: Vec<String>,
    /// More checks exist than were given: the counts cover only the pages read.
    pub partial: bool,
}

/// How one check or status counts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tally {
    Passed,
    Failed,
    Pending,
    Other,
}

fn tally_run(run: &CheckRun) -> Tally {
    if run.status != RunStatus::Completed {
        return Tally::Pending;
    }
    match run.conclusion {
        Some(Conclusion::Success) => Tally::Passed,
        Some(Conclusion::Failure | Conclusion::TimedOut | Conclusion::ActionRequired) => {
            Tally::Failed
        }
        _ => Tally::Other,
    }
}

fn tally_status(status: &CommitStatus) -> Tally {
    match status.state {
        StatusState::Success => Tally::Passed,
        StatusState::Failure | StatusState::Error => Tally::Failed,
        StatusState::Pending => Tally::Pending,
        StatusState::Other => Tally::Other,
    }
}

/// Counts check runs and commit statuses together. A combined status with no statuses adds
/// nothing (GitHub reads it as `pending`, which it is not). A run that is not finished is
/// pending; a failed, timed out or action-required check is failed; skipped, neutral,
/// cancelled and unknown results are `other`.
#[must_use]
pub fn count(runs: &CheckRuns, statuses: &CombinedStatus) -> Counts {
    let tallies = runs
        .runs
        .iter()
        .map(|run| (run.name.as_str(), tally_run(run)))
        .chain(
            statuses
                .statuses
                .iter()
                .map(|status| (status.context.as_str(), tally_status(status))),
        );
    let mut counts = Counts {
        state: ChecksState::None,
        passed: 0,
        failed: 0,
        pending: 0,
        other: 0,
        failing: Vec::new(),
        partial: runs.total_count > runs.runs.len() as u64
            || statuses.total_count > statuses.statuses.len() as u64,
    };
    for (name, tally) in tallies {
        match tally {
            Tally::Passed => counts.passed += 1,
            Tally::Pending => counts.pending += 1,
            Tally::Other => counts.other += 1,
            Tally::Failed => {
                counts.failed += 1;
                if counts.failing.len() < MOST_FAILING {
                    counts
                        .failing
                        .push(name.chars().take(NAME_LENGTH).collect());
                }
            }
        }
    }
    counts.state = if counts.failed > 0 {
        ChecksState::Failed
    } else if counts.pending > 0 {
        ChecksState::Pending
    } else if counts.passed + counts.other > 0 {
        ChecksState::Passed
    } else {
        ChecksState::None
    };
    counts
}
