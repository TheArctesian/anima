//! `anima repo branches <repository>`: branches by name, with pages (CMD-026).

use super::query::{History, Query};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::paging::{CommandLine, fetch, note_next, page_size};
use crate::projects::repository::resolve;

/// The hub's largest page of branches.
const MAX_PAGE: i64 = 100;

#[derive(Debug, clap::Args)]
pub struct BranchesArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// Branches per page, 1 to 100.
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
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

impl BranchesArgs {
    /// Prints the branches as a table or, with `--json`, as the K1 `BranchConnection`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command = CommandLine::new("anima repo branches", self.limit).word(&self.repository);
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let asked = |after| Query {
            repository: repository.clone(),
            history: History::Branches,
            first: page_size(self.limit, self.all, MAX_PAGE),
            after,
            fresh: self.fresh,
        };
        let (hub, asked) = (&hub, &asked);
        let branches = fetch(self.after.clone(), self.all, move |after| async move {
            Ok(asked(after).run(hub).await?)
        })
        .await?;
        invocation.output.data(&branches, render::branches)?;
        note_next(invocation, &branches, "branches", &command);
        Ok(())
    }
}
