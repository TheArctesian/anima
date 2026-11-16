//! `anima issue list <repository>`: issues, newest updated first, with pages (CMD-026).

use super::documents::{IssueList, issue_list};
use super::render;
use crate::connection_page;
use crate::failure::Failure;
use crate::forge::StateArg;
use crate::invocation::Invocation;
use crate::paging::{CommandLine, fetch, note_next, page_size};
use crate::projects::repository::resolve;

/// The hub's largest page of issues.
const MAX_PAGE: i64 = 100;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// Which issues to list.
    #[arg(long, value_enum, default_value_t)]
    state: StateArg,
    /// Issues per page, 1 to 100.
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

connection_page!(issue_list::IssueListIssues);

const fn state(arg: StateArg) -> issue_list::OpenOrClosed {
    match arg {
        StateArg::Open => issue_list::OpenOrClosed::OPEN,
        StateArg::Closed => issue_list::OpenOrClosed::CLOSED,
        StateArg::All => issue_list::OpenOrClosed::ALL,
    }
}

impl ListArgs {
    /// Prints the issues as a table or, with `--json`, as the K1 `IssueConnection`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command = CommandLine::new("anima issue list", self.limit)
            .word(&self.repository)
            .flag("--state", Some(self.state.word()));
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let (hub, repository, fresh, arg) = (&hub, &repository, self.fresh, self.state);
        let issues = fetch(self.after, self.all, move |after| async move {
            let variables = issue_list::Variables {
                repository: repository.clone(),
                state: Some(state(arg)),
                first,
                after,
                fresh: Some(fresh),
            };
            Ok(hub.run::<IssueList>(variables).await?.issues)
        })
        .await?;
        invocation.output.data(&issues, render::list)?;
        note_next(invocation, &issues, "issues", &command);
        Ok(())
    }
}
