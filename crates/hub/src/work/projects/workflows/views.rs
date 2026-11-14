//! The K1 shapes of workflows and their runs, from the provider's types. Pure.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, SimpleObject};
use chrono::{DateTime, Utc};
use provider_github::live::{
    runs::{Conclusion, Run, RunStatus, StatusFilter},
    workflows::{Workflow, WorkflowState},
};

use crate::{
    paging::PageEnd,
    work::projects::forge::{Asked, ForgeRead, int, text},
};

/// The most characters of a run's commit title.
const COMMIT_TITLE: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum WorkflowRunStatus {
    Queued,
    InProgress,
    Completed,
}

impl WorkflowRunStatus {
    /// The provider's filter for this status.
    #[must_use]
    pub const fn filter(self) -> StatusFilter {
        match self {
            Self::Queued => StatusFilter::Queued,
            Self::InProgress => StatusFilter::InProgress,
            Self::Completed => StatusFilter::Completed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum WorkflowConclusion {
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

#[derive(SimpleObject)]
#[graphql(name = "Workflow")]
pub struct WorkflowView {
    /// GitHub's workflow number, as digits.
    id: ID,
    /// Text from GitHub.
    name: String,
    /// For example .github/workflows/ci.yml. Text from GitHub.
    path: String,
    /// False: the workflow is disabled on GitHub.
    active: bool,
    url: String,
}

/// The workflows of a repository by name, then path; at most the first page of GitHub's.
#[must_use]
pub fn workflows(mut found: Vec<Workflow>) -> Vec<WorkflowView> {
    found.sort_by(|a, b| (a.name.to_lowercase(), &a.path).cmp(&(b.name.to_lowercase(), &b.path)));
    found
        .into_iter()
        .map(|workflow| WorkflowView {
            id: ID(workflow.id.to_string()),
            name: workflow.name,
            path: workflow.path,
            active: workflow.state == WorkflowState::Active,
            url: workflow.url,
        })
        .collect()
}

/// GitHub's run status as K1's: the states before a run starts are `QUEUED`, and a status
/// the hub does not know is `IN_PROGRESS` (not finished, so nothing claims it ended).
#[must_use]
pub const fn run_status(status: RunStatus) -> WorkflowRunStatus {
    match status {
        RunStatus::Queued | RunStatus::Waiting | RunStatus::Requested | RunStatus::Pending => {
            WorkflowRunStatus::Queued
        }
        RunStatus::InProgress | RunStatus::Other => WorkflowRunStatus::InProgress,
        RunStatus::Completed => WorkflowRunStatus::Completed,
    }
}

#[must_use]
pub const fn conclusion(conclusion: Conclusion) -> WorkflowConclusion {
    match conclusion {
        Conclusion::Success => WorkflowConclusion::Success,
        Conclusion::Failure => WorkflowConclusion::Failure,
        Conclusion::Cancelled => WorkflowConclusion::Cancelled,
        Conclusion::Skipped => WorkflowConclusion::Skipped,
        Conclusion::TimedOut => WorkflowConclusion::TimedOut,
        Conclusion::ActionRequired => WorkflowConclusion::ActionRequired,
        Conclusion::Neutral => WorkflowConclusion::Neutral,
        Conclusion::Stale => WorkflowConclusion::Stale,
        Conclusion::Other => WorkflowConclusion::Other,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "WorkflowRun", complex)]
#[expect(
    clippy::struct_excessive_bools,
    clippy::doc_markdown,
    reason = "K1 fixes the fields of WorkflowRun and their doc strings"
)]
pub struct RunView {
    repository: ID,
    /// GitHub's run number across the repository, as digits (it does not fit Int).
    id: ID,
    workflow_id: ID,
    /// The workflow's name. Text from GitHub.
    name: String,
    /// The count of runs of this workflow.
    run_number: i32,
    attempt: i32,
    /// For example push, pull_request, workflow_dispatch.
    event: String,
    status: WorkflowRunStatus,
    /// Null until COMPLETED.
    conclusion: Option<WorkflowConclusion>,
    /// Null for a run with no branch. Text from GitHub.
    branch: Option<String>,
    /// 40 lower-case hex digits. Each confirmation shows it (PRJ-034).
    head_sha: String,
    /// The first line of the commit message, at most 200 characters. Text from GitHub.
    commit_title: Option<String>,
    /// Text from GitHub.
    actor_login: Option<String>,
    /// True: the head commit is in another repository. The hub does not run such a run again (PRO-013).
    from_fork: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    url: String,
    /// COMPLETED and not fromFork.
    rerunnable: bool,
    /// rerunnable, and the conclusion is FAILURE, CANCELLED or TIMED_OUT.
    failed_jobs_rerunnable: bool,
    /// QUEUED or IN_PROGRESS.
    cancellable: bool,
    /// The run GitHub numbers, for its jobs.
    #[graphql(skip)]
    pub github_id: u64,
    #[graphql(skip)]
    pub asked: Asked,
}

/// The K1 run of `run`, read for `asked`.
#[must_use]
pub fn run(run: Run, asked: Asked) -> RunView {
    let status = run_status(run.status);
    let completed = status == WorkflowRunStatus::Completed;
    let conclusion = run.conclusion.filter(|_| completed).map(conclusion);
    let rerunnable = completed && !run.from_fork;
    RunView {
        repository: ID(asked.target.repository.to_string()),
        id: ID(run.id.to_string()),
        workflow_id: ID(run.workflow_id.to_string()),
        name: run.name,
        run_number: int(run.run_number),
        attempt: int(u64::from(run.attempt)),
        event: run.event,
        status,
        conclusion,
        branch: run.branch,
        head_sha: run.head_sha,
        commit_title: run
            .commit_title
            .map(|title| text::capped(title, COMMIT_TITLE)),
        actor_login: run.actor_login,
        from_fork: run.from_fork,
        created_at: run.created_at,
        updated_at: run.updated_at,
        started_at: run.started_at,
        url: run.url,
        rerunnable,
        failed_jobs_rerunnable: rerunnable
            && matches!(
                conclusion,
                Some(
                    WorkflowConclusion::Failure
                        | WorkflowConclusion::Cancelled
                        | WorkflowConclusion::TimedOut
                )
            ),
        cancellable: !completed,
        github_id: run.id,
        asked,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "WorkflowRunEdge")]
pub struct RunEdge {
    pub cursor: String,
    pub node: RunView,
}

#[derive(SimpleObject)]
#[graphql(name = "WorkflowRunConnection")]
pub struct RunConnection {
    pub edges: Vec<RunEdge>,
    pub page_info: PageEnd,
    pub read: ForgeRead,
}
