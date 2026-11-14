//! `anima workflow runs <repository>`: runs, newest first, with pages (CMD-026).

use super::query::{Query, RunStatus};
use super::render;
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
    status: Option<RunStatus>,
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
        let asked = |after| Query {
            repository: repository.clone(),
            branch: self.branch.clone(),
            status: self.status,
            first: page_size(self.limit, self.all, MAX_PAGE),
            after,
            fresh: self.fresh,
            jobs: self.jobs,
        };
        let (hub, asked) = (&hub, &asked);
        let runs = fetch(self.after.clone(), self.all, move |after| async move {
            Ok(asked(after).run(hub).await?)
        })
        .await?;
        invocation.output.data(&runs, render::runs)?;
        note_next(invocation, &runs, "runs", &command);
        Ok(())
    }
}
