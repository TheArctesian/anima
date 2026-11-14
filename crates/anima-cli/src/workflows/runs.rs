//! `anima workflow runs <repository>`: runs, newest first, with pages (CMD-026).

use super::documents::{WorkflowRunList, workflow_run_list};
use super::render;
use crate::connection_page;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::paging::{CommandLine, fetch, note_next, page_size};
use crate::projects::repository::resolve;

/// The hub's largest page of runs.
const MAX_PAGE: i64 = 50;

#[derive(Debug, clap::Args)]
pub struct RunsArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// Only runs on this branch.
    #[arg(long)]
    branch: Option<String>,
    /// Only runs with this status.
    #[arg(long, value_enum)]
    status: Option<StatusArg>,
    /// Also count each run's jobs (costs a request for each run).
    #[arg(long)]
    jobs: bool,
    /// Runs per page, 1 to 50.
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
    /// Skip the hub's cache and ask GitHub again.
    #[arg(long)]
    fresh: bool,
}

/// K1 `WorkflowRunStatus`.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum StatusArg {
    Queued,
    InProgress,
    Completed,
}

impl StatusArg {
    const fn k1(self) -> workflow_run_list::WorkflowRunStatus {
        match self {
            Self::Queued => workflow_run_list::WorkflowRunStatus::QUEUED,
            Self::InProgress => workflow_run_list::WorkflowRunStatus::IN_PROGRESS,
            Self::Completed => workflow_run_list::WorkflowRunStatus::COMPLETED,
        }
    }
}

connection_page!(workflow_run_list::WorkflowRunListWorkflowRuns);

impl RunsArgs {
    /// Prints the runs as a table or, with `--json`, as the K1 `WorkflowRunConnection`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command = CommandLine::new("anima workflow runs", self.limit)
            .word(&self.repository)
            .flag("--branch", self.branch.as_deref())
            .choices("--status", &self.status)
            .switch("--jobs", self.jobs);
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let (hub, repository, branch) = (&hub, &repository, &self.branch);
        let (jobs, fresh, status) = (self.jobs, self.fresh, self.status);
        let runs = fetch(self.after, self.all, move |after| async move {
            let variables = workflow_run_list::Variables {
                repository: repository.clone(),
                branch: branch.clone(),
                status: status.map(StatusArg::k1),
                first,
                after,
                fresh: Some(fresh),
                jobs,
            };
            Ok(hub.run::<WorkflowRunList>(variables).await?.workflow_runs)
        })
        .await?;
        invocation.output.data(&runs, render::runs)?;
        note_next(invocation, &runs, "runs", &command);
        Ok(())
    }
}
