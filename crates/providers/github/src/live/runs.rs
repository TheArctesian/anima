//! Workflows, their runs and the jobs of a run (PRJ-031).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use source_exchange::{Problem, Request, Response};

use super::{User, get, login, read};

const SUBJECT: &str = "workflow runs";

/// A run, job or check status, an open string: anything not named here is `Other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunStatus {
    Queued,
    InProgress,
    Completed,
    Waiting,
    Requested,
    Pending,
    Other,
}

impl RunStatus {
    pub(super) fn named(name: Option<&str>) -> Self {
        match name {
            Some("queued") => Self::Queued,
            Some("in_progress") => Self::InProgress,
            Some("completed") => Self::Completed,
            Some("waiting") => Self::Waiting,
            Some("requested") => Self::Requested,
            Some("pending") => Self::Pending,
            _ => Self::Other,
        }
    }
}

/// A run, job or check conclusion, an open string: anything not named here is `Other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conclusion {
    Success,
    Failure,
    Cancelled,
    Skipped,
    TimedOut,
    ActionRequired,
    Neutral,
    Stale,
    Other,
}

impl Conclusion {
    pub(super) fn named(name: &str) -> Self {
        match name {
            "success" => Self::Success,
            "failure" => Self::Failure,
            "cancelled" => Self::Cancelled,
            "skipped" => Self::Skipped,
            "timed_out" => Self::TimedOut,
            "action_required" => Self::ActionRequired,
            "neutral" => Self::Neutral,
            "stale" => Self::Stale,
            _ => Self::Other,
        }
    }
}

/// The one value of the `status` filter: GitHub takes a status or a conclusion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusFilter {
    Completed,
    ActionRequired,
    Cancelled,
    Failure,
    Neutral,
    Skipped,
    Stale,
    Success,
    TimedOut,
    InProgress,
    Queued,
    Requested,
    Waiting,
    Pending,
}

impl StatusFilter {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::ActionRequired => "action_required",
            Self::Cancelled => "cancelled",
            Self::Failure => "failure",
            Self::Neutral => "neutral",
            Self::Skipped => "skipped",
            Self::Stale => "stale",
            Self::Success => "success",
            Self::TimedOut => "timed_out",
            Self::InProgress => "in_progress",
            Self::Queued => "queued",
            Self::Requested => "requested",
            Self::Waiting => "waiting",
            Self::Pending => "pending",
        }
    }
}

/// What a run list is narrowed to. A filtered list ends at 1,000 results (GitHub's cap).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunFilter<'a> {
    pub branch: Option<&'a str>,
    pub status: Option<StatusFilter>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub id: u64,
    pub workflow_id: u64,
    /// The workflow's name; the run's title when GitHub gives no name.
    pub name: String,
    pub run_number: u64,
    pub attempt: u32,
    pub event: String,
    pub status: RunStatus,
    pub conclusion: Option<Conclusion>,
    pub branch: Option<String>,
    pub head_sha: String,
    /// The first line of the head commit's message, whole.
    pub commit_title: Option<String>,
    pub actor_login: Option<String>,
    /// The head commit is in another repository, or that repository is gone.
    pub from_fork: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub url: String,
}

#[derive(Deserialize)]
struct RepoId {
    id: u64,
}

#[derive(Deserialize)]
struct HeadCommit {
    message: Option<String>,
}

#[derive(Deserialize)]
struct RawRun {
    id: u64,
    workflow_id: u64,
    name: Option<String>,
    display_title: Option<String>,
    run_number: u64,
    run_attempt: Option<u32>,
    event: String,
    status: Option<String>,
    conclusion: Option<String>,
    head_branch: Option<String>,
    head_sha: String,
    head_commit: Option<HeadCommit>,
    actor: Option<User>,
    repository: Option<RepoId>,
    head_repository: Option<RepoId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    run_started_at: Option<DateTime<Utc>>,
    html_url: String,
}

impl RawRun {
    fn into_run(self) -> Run {
        let from_fork = !matches!(
            (&self.head_repository, &self.repository),
            (Some(head), Some(base)) if head.id == base.id
        );
        Run {
            id: self.id,
            workflow_id: self.workflow_id,
            name: self.name.or(self.display_title).unwrap_or_default(),
            run_number: self.run_number,
            attempt: self.run_attempt.unwrap_or(1),
            event: self.event,
            status: RunStatus::named(self.status.as_deref()),
            conclusion: self.conclusion.as_deref().map(Conclusion::named),
            branch: self.head_branch,
            head_sha: self.head_sha,
            commit_title: self
                .head_commit
                .and_then(|commit| commit.message)
                .map(|message| message.lines().next().unwrap_or_default().to_owned()),
            actor_login: self.actor.map(|actor| login(Some(actor))),
            from_fork,
            created_at: self.created_at,
            updated_at: self.updated_at,
            started_at: self.run_started_at,
            url: self.html_url,
        }
    }
}

#[derive(Deserialize)]
struct RunList {
    workflow_runs: Vec<RawRun>,
}

/// `GET /repos/{o}/{r}/actions/runs`, 100 to a page, narrowed by `filter`.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn list_request(
    repository: &str,
    filter: RunFilter<'_>,
    page: u32,
) -> Result<Request, Problem> {
    let mut filters = Vec::new();
    if let Some(branch) = filter.branch {
        filters.push(("branch", branch));
    }
    if let Some(status) = filter.status {
        filters.push(("status", status.as_str()));
    }
    get(repository, "/actions/runs", &filters, Some(page))
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of runs.
pub fn list_parse(response: &Response<'_>) -> Result<Vec<Run>, Problem> {
    let list: RunList = read(response, SUBJECT)?;
    Ok(list
        .workflow_runs
        .into_iter()
        .map(RawRun::into_run)
        .collect())
}

/// `GET /repos/{o}/{r}/actions/runs/{run_id}`.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn one_request(repository: &str, run_id: u64) -> Result<Request, Problem> {
    get(repository, &format!("/actions/runs/{run_id}"), &[], None)
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a run.
pub fn one_parse(response: &Response<'_>) -> Result<Run, Problem> {
    read::<RawRun>(response, SUBJECT).map(RawRun::into_run)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub status: RunStatus,
    pub conclusion: Option<Conclusion>,
}

/// The jobs of a run: the latest attempt's, with GitHub's total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Jobs {
    pub total_count: u64,
    pub jobs: Vec<Job>,
}

#[derive(Deserialize)]
struct RawJob {
    id: u64,
    name: String,
    status: Option<String>,
    conclusion: Option<String>,
}

#[derive(Deserialize)]
struct JobList {
    total_count: u64,
    jobs: Vec<RawJob>,
}

/// `GET /repos/{o}/{r}/actions/runs/{run_id}/jobs`, 100 to a page.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn jobs_request(repository: &str, run_id: u64, page: u32) -> Result<Request, Problem> {
    get(
        repository,
        &format!("/actions/runs/{run_id}/jobs"),
        &[],
        Some(page),
    )
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of jobs.
pub fn jobs_parse(response: &Response<'_>) -> Result<Jobs, Problem> {
    let list: JobList = read(response, "workflow jobs")?;
    Ok(Jobs {
        total_count: list.total_count,
        jobs: list
            .jobs
            .into_iter()
            .map(|job| Job {
                id: job.id,
                name: job.name,
                status: RunStatus::named(job.status.as_deref()),
                conclusion: job.conclusion.as_deref().map(Conclusion::named),
            })
            .collect(),
    })
}

/// GitHub's workflow `state`, an open string: anything not named here is `Other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowState {
    Active,
    Deleted,
    DisabledFork,
    DisabledInactivity,
    DisabledManually,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    /// For example `.github/workflows/ci.yml`.
    pub path: String,
    pub state: WorkflowState,
    pub url: String,
}

#[derive(Deserialize)]
struct RawWorkflow {
    id: u64,
    name: String,
    path: String,
    state: String,
    html_url: String,
}

#[derive(Deserialize)]
struct WorkflowList {
    workflows: Vec<RawWorkflow>,
}

/// `GET /repos/{o}/{r}/actions/workflows`, 100 to a page.
///
/// # Errors
/// A [`Problem`] when the repository name cannot go into a path.
pub fn workflows_request(repository: &str, page: u32) -> Result<Request, Problem> {
    get(repository, "/actions/workflows", &[], Some(page))
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a list of workflows.
pub fn workflows_parse(response: &Response<'_>) -> Result<Vec<Workflow>, Problem> {
    let list: WorkflowList = read(response, "workflows")?;
    Ok(list
        .workflows
        .into_iter()
        .map(|workflow| Workflow {
            id: workflow.id,
            name: workflow.name,
            path: workflow.path,
            state: match workflow.state.as_str() {
                "active" => WorkflowState::Active,
                "deleted" => WorkflowState::Deleted,
                "disabled_fork" => WorkflowState::DisabledFork,
                "disabled_inactivity" => WorkflowState::DisabledInactivity,
                "disabled_manually" => WorkflowState::DisabledManually,
                _ => WorkflowState::Other,
            },
            url: workflow.html_url,
        })
        .collect())
}
