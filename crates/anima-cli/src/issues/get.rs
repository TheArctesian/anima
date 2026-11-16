//! `anima issue get <repository> <number>`.

use super::documents::{IssueGet, issue_get};
use super::render;
use crate::failure::{Failure, Hint};
use crate::forge::comments::{CommentArgs, Thread};
use crate::invocation::Invocation;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct GetArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The issue's number.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    number: i64,
    #[command(flatten)]
    comments: CommentArgs,
    /// Skip the hub's cache and ask GitHub again.
    #[arg(long)]
    fresh: bool,
}

impl GetArgs {
    /// Prints the issue or, with `--json`, the K1 `Issue` (with a `comments` connection under
    /// `--comments`).
    ///
    /// # Errors
    /// [`Failure::NotFound`] when the repository has no such issue; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let variables = issue_get::Variables {
            repository: repository.clone(),
            number: self.number,
            fresh: Some(self.fresh),
        };
        let issue =
            hub.run::<IssueGet>(variables)
                .await?
                .issue
                .ok_or_else(|| Failure::NotFound {
                    message: format!("{} has no issue number {}.", self.repository, self.number),
                    hint: Some(Hint::run(format!("anima issue list {}", self.repository))),
                })?;
        let thread = Thread {
            repository,
            number: self.number,
            fresh: self.fresh,
        };
        let shown = serde_json::to_value(&issue).unwrap_or_default();
        let shown = self.comments.add_to(&hub, shown, &thread).await?;
        invocation.output.data(&shown, render::record)
    }
}
