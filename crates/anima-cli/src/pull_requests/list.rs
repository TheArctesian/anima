//! `anima pr list <repository>`: pull requests, newest updated first, with pages (CMD-026).

use super::query::Query;
use super::render;
use crate::failure::Failure;
use crate::forge::StateArg;
use crate::invocation::Invocation;
use crate::paging::{CommandLine, fetch, note_next, page_size};
use crate::projects::repository::resolve;

/// The hub's largest page of pull requests.
const MAX_PAGE: i64 = 50;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// Which pull requests to list.
    #[arg(long, value_enum, default_value_t)]
    state: StateArg,
    /// Also ask GitHub whether each can merge and how its checks stand (costs requests).
    #[arg(long)]
    checks: bool,
    /// Pull requests per page, 1 to 50.
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

impl ListArgs {
    /// Prints the pull requests as a table or, with `--json`, as the K1
    /// `PullRequestConnection`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command = CommandLine::new("anima pr list", self.limit)
            .word(&self.repository)
            .flag("--state", Some(self.state.word()))
            .switch("--checks", self.checks);
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let asked = |after| Query {
            repository: repository.clone(),
            state: self.state,
            first: page_size(self.limit, self.all, MAX_PAGE),
            after,
            fresh: self.fresh,
            checked: self.checks,
        };
        let (hub, asked) = (&hub, &asked);
        let pulls = fetch(self.after.clone(), self.all, move |after| async move {
            Ok(asked(after).run(hub).await?)
        })
        .await?;
        invocation.output.data(&pulls, render::list)?;
        note_next(invocation, &pulls, "pull requests", &command);
        Ok(())
    }
}
