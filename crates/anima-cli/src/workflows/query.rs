//! One page of workflow runs, asked the same way by the command and by the MCP tool.

use super::documents::{
    WorkflowRunList, WorkflowRunListJobs, workflow_run_list, workflow_run_list_jobs,
};
use crate::forge::rows::Rows;
use crate::hub::{Hub, HubError};

/// K1 `WorkflowRunStatus`.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum RunStatus {
    Queued,
    InProgress,
    Completed,
}

/// What `workflowRuns` is asked: `jobs` adds the job counts, which cost a GitHub request for
/// each run (PRJ-034).
#[derive(Debug)]
pub struct Query {
    pub repository: String,
    pub branch: Option<String>,
    pub status: Option<RunStatus>,
    pub first: i64,
    pub after: Option<String>,
    pub fresh: bool,
    pub jobs: bool,
}

impl Query {
    /// The page the hub reads from GitHub.
    ///
    /// # Errors
    /// [`HubError`] for a refusal or a failed request.
    pub async fn run(&self, hub: &Hub) -> Result<Rows, HubError> {
        let (repository, branch) = (self.repository.clone(), self.branch.clone());
        let (first, after, fresh) = (Some(self.first), self.after.clone(), Some(self.fresh));
        if self.jobs {
            let status = self.status.map(|status| match status {
                RunStatus::Queued => workflow_run_list_jobs::WorkflowRunStatus::QUEUED,
                RunStatus::InProgress => workflow_run_list_jobs::WorkflowRunStatus::IN_PROGRESS,
                RunStatus::Completed => workflow_run_list_jobs::WorkflowRunStatus::COMPLETED,
            });
            let variables = workflow_run_list_jobs::Variables {
                repository,
                branch,
                status,
                first,
                after,
                fresh,
            };
            let page = hub.run::<WorkflowRunListJobs>(variables).await?;
            return Ok(Rows::new(&page.workflow_runs));
        }
        let status = self.status.map(|status| match status {
            RunStatus::Queued => workflow_run_list::WorkflowRunStatus::QUEUED,
            RunStatus::InProgress => workflow_run_list::WorkflowRunStatus::IN_PROGRESS,
            RunStatus::Completed => workflow_run_list::WorkflowRunStatus::COMPLETED,
        });
        let variables = workflow_run_list::Variables {
            repository,
            branch,
            status,
            first,
            after,
            fresh,
        };
        let page = hub.run::<WorkflowRunList>(variables).await?;
        Ok(Rows::new(&page.workflow_runs))
    }
}
