//! `anima workflow list <repository>`.

use super::documents::{WorkflowList, workflow_list};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// Skip the hub's cache and ask GitHub again.
    #[arg(long)]
    fresh: bool,
}

impl ListArgs {
    /// Prints the workflows as a table or, with `--json`, as a list of K1 `Workflow`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let variables = workflow_list::Variables {
            repository,
            fresh: Some(self.fresh),
        };
        let workflows = hub.run::<WorkflowList>(variables).await?.workflows;
        invocation.output.data(&workflows, render::workflows)
    }
}
