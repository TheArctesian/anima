//! `anima pr get <repository> <number>`.

use super::documents::{PullRequestGet, pull_request_get};
use super::render;
use crate::failure::{Failure, Hint};
use crate::forge::comments::{CommentArgs, Thread};
use crate::invocation::Invocation;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct GetArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The pull request's number.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    number: i64,
    #[command(flatten)]
    comments: CommentArgs,
    /// Skip the hub's cache and ask GitHub again.
    #[arg(long)]
    fresh: bool,
}

impl GetArgs {
    /// Prints the pull request or, with `--json`, the K1 `PullRequest` (with a `comments`
    /// connection under `--comments`).
    ///
    /// # Errors
    /// [`Failure::NotFound`] when the repository has no such pull request; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let variables = pull_request_get::Variables {
            repository: repository.clone(),
            number: self.number,
            fresh: Some(self.fresh),
        };
        let pull = hub
            .run::<PullRequestGet>(variables)
            .await?
            .pull_request
            .ok_or_else(|| Failure::NotFound {
                message: format!(
                    "{} has no pull request number {}.",
                    self.repository, self.number
                ),
                hint: Some(Hint::run(format!("anima pr list {}", self.repository))),
            })?;
        let thread = Thread {
            repository,
            number: self.number,
            fresh: self.fresh,
        };
        let shown = serde_json::to_value(&pull).unwrap_or_default();
        let shown = self.comments.add_to(&hub, shown, &thread).await?;
        invocation.output.data(&shown, render::record)
    }
}
