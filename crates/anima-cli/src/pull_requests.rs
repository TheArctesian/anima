//! `anima pr`: the pull requests of a repository the hub reads live from GitHub (PRJ-030,
//! PRJ-031, PRJ-033).

pub mod documents;
pub mod get;
pub mod list;
pub mod render;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum PullRequestCommand {
    /// List the pull requests of a repository, newest updated first.
    List(list::ListArgs),
    /// Show one pull request: fields, checks, whether it can merge, and with `--comments` its
    /// comments.
    Get(get::GetArgs),
}

impl PullRequestCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or pull request, or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
        }
    }
}
