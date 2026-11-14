//! `anima pr list <repository>`: pull requests, newest updated first, with pages (CMD-026).

use super::documents::{PullRequestList, pull_request_list};
use super::render;
use crate::connection_page;
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

connection_page!(pull_request_list::PullRequestListPullRequests);

const fn state(arg: StateArg) -> pull_request_list::OpenOrClosed {
    match arg {
        StateArg::Open => pull_request_list::OpenOrClosed::OPEN,
        StateArg::Closed => pull_request_list::OpenOrClosed::CLOSED,
        StateArg::All => pull_request_list::OpenOrClosed::ALL,
    }
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
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let (hub, repository, detail, fresh) = (&hub, &repository, self.checks, self.fresh);
        let pulls = fetch(self.after, self.all, move |after| async move {
            let variables = pull_request_list::Variables {
                repository: repository.clone(),
                state: Some(state(self.state)),
                first,
                after,
                fresh: Some(fresh),
                detail,
            };
            Ok(hub.run::<PullRequestList>(variables).await?.pull_requests)
        })
        .await?;
        invocation.output.data(&pulls, render::list)?;
        note_next(invocation, &pulls, "pull requests", &command);
        Ok(())
    }
}
