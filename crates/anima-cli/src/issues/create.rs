//! `anima issue create <repository>`: makes an issue on GitHub through the hub (OD-E14,
//! PRJ-028).

use std::path::PathBuf;

use super::documents::{IssueCreate, issue_create};
use super::render;
use crate::failure::Failure;
use crate::forge::body;
use crate::invocation::Invocation;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The title, 1 to 256 characters.
    #[arg(long)]
    title: String,
    /// The body in Markdown, from a file or `-` for standard input.
    #[arg(long, value_name = "PATH|-")]
    body_file: Option<PathBuf>,
    /// A label to set; repeat for more (at most 20).
    #[arg(long = "label", value_name = "NAME")]
    labels: Vec<String>,
}

impl CreateArgs {
    /// Prints the new issue's number and link or, with `--json`, the K1 `Issue` without its
    /// body. The hub does not repeat it: after `UNAVAILABLE` the issue may exist.
    ///
    /// # Errors
    /// [`Failure::Input`] for a body that cannot be read; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let body = match &self.body_file {
            Some(path) => Some(body::read(invocation.prompter, path, "the issue's body")?),
            None => None,
        };
        let hub = invocation.hub()?;
        let repository = resolve(&hub, &self.repository).await?;
        let labels = (!self.labels.is_empty()).then_some(self.labels);
        let variables = issue_create::Variables {
            input: issue_create::NewIssueInput {
                repository,
                title: self.title,
                body,
                labels,
            },
        };
        let issue = hub.run::<IssueCreate>(variables).await?.create_issue;
        invocation.output.data(&issue, render::changed)
    }
}
