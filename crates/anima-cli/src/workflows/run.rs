//! `anima workflow run <repository> <run>`: one run with its jobs.

use super::documents::{WorkflowRunGet, workflow_run_get};
use super::render;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The run's ID, as `anima workflow runs` shows it.
    run: String,
    /// Skip the hub's cache and ask GitHub again.
    #[arg(long)]
    fresh: bool,
}

impl RunArgs {
    /// Prints the run or, with `--json`, the K1 `WorkflowRun`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] when the repository has no such run; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let variables = workflow_run_get::Variables {
            repository,
            run_id: self.run.clone(),
            fresh: Some(self.fresh),
        };
        let run = hub
            .run::<WorkflowRunGet>(variables)
            .await?
            .workflow_run
            .ok_or_else(|| Failure::NotFound {
                message: format!("{} has no run {}.", self.repository, self.run),
                hint: Some(Hint::run(format!(
                    "anima workflow runs {}",
                    self.repository
                ))),
            })?;
        invocation.output.data(&run, render::run)
    }
}
