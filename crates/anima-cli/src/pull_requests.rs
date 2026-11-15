//! `anima pr`: the pull requests of a repository the hub reads live from GitHub, and merging
//! them (PRJ-029 to PRJ-031, PRJ-033, PRO-013).

pub mod documents;
pub mod get;
pub mod list;
pub mod merge;
pub mod query;
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
    /// Merge one pull request, or up to 20 one after another. Needs the operator's sign-in.
    Merge(merge::MergeArgs),
}

impl PullRequestCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or pull request, a declined confirmation, or a
    /// hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
            Self::Merge(args) => args.run(invocation).await,
        }
    }
}
