//! `anima repo commits <repository>`: commits, newest first, with pages (CMD-026).

use super::query::{History, Query};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::paging::{CommandLine, fetch, note_next, page_size};
use crate::projects::repository::resolve;

/// The hub's largest page of commits.
const MAX_PAGE: i64 = 100;

#[derive(Debug, clap::Args)]
pub struct CommitsArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// A branch name or a 40-digit commit to read from; default the repository's default branch.
    #[arg(long = "ref")]
    reference: Option<String>,
    /// Also count each commit's checks (costs two GitHub requests for each commit).
    #[arg(long)]
    checks: bool,
    /// Commits per page, 1 to 100.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
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

impl CommitsArgs {
    /// Prints the commits as a table or, with `--json`, as the K1 `CommitConnection`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command = CommandLine::new("anima repo commits", self.limit)
            .word(&self.repository)
            .flag("--ref", self.reference.as_deref())
            .switch("--checks", self.checks);
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let asked = |after| Query {
            repository: repository.clone(),
            history: History::Commits {
                reference: self.reference.clone(),
                checked: self.checks,
            },
            first: page_size(self.limit, self.all, MAX_PAGE),
            after,
            fresh: self.fresh,
        };
        let (hub, asked) = (&hub, &asked);
        let commits = fetch(self.after.clone(), self.all, move |after| async move {
            Ok(asked(after).run(hub).await?)
        })
        .await?;
        invocation.output.data(&commits, render::commits)?;
        note_next(invocation, &commits, "commits", &command);
        Ok(())
    }
}
