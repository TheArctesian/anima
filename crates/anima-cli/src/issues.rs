//! `anima issue`: the issues of a repository the hub reads live from GitHub (PRJ-030, PRJ-031).

pub mod documents;
pub mod get;
pub mod list;
pub mod render;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum IssueCommand {
    /// List the issues of a repository, newest updated first (pull requests are not in it).
    List(list::ListArgs),
    /// Show one issue: fields and body, and with `--comments` its comments.
    Get(get::GetArgs),
}

impl IssueCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or issue, or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
        }
    }
}
