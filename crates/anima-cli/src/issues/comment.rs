//! `anima issue comment <repository> <number>`: adds a comment to an issue or a pull request
//! on GitHub through the hub (OD-E14, PRJ-028).

use std::path::PathBuf;

use super::documents::{IssueCommentAdd, issue_comment_add};
use crate::failure::Failure;
use crate::forge::body;
use crate::forge::render::text;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct AddCommentArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The number of the issue or the pull request.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    number: i64,
    /// The comment in Markdown, from a file or `-` for standard input.
    #[arg(long, value_name = "PATH|-")]
    body_file: PathBuf,
}

impl AddCommentArgs {
    /// Prints the new comment's ID and link or, with `--json`, the K1 `IssueComment` without
    /// its body. The hub does not repeat it: after `UNAVAILABLE` the comment may exist.
    ///
    /// # Errors
    /// [`Failure::Input`] for a body that cannot be read; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let body = body::read(invocation.prompter, &self.body_file, "the comment")?;
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let variables = issue_comment_add::Variables {
            repository,
            number: self.number,
            body,
        };
        let comment = hub
            .run::<IssueCommentAdd>(variables)
            .await?
            .add_issue_comment;
        invocation.output.data(&comment, |comment| {
            let shown = serde_json::to_value(comment).unwrap_or_default();
            let mut table = Table::record();
            table.pair("Comment", text(&shown, "id"));
            table.pair("Link", text(&shown, "url"));
            table
        })
    }
}
