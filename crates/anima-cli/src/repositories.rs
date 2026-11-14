//! `anima repo`: the history of a repository the hub reads live from GitHub: commits,
//! branches and releases (PRJ-036 to PRJ-038).

pub mod branches;
pub mod commits;
pub mod documents;
pub mod query;
pub mod releases;
pub mod render;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum RepositoryCommand {
    /// List the commits of a branch or a commit, newest first.
    Commits(commits::CommitsArgs),
    /// List the branches of a repository, by name.
    Branches(branches::BranchesArgs),
    /// List the releases on GitHub of a repository, newest first.
    Releases(releases::ReleasesArgs),
}

impl RepositoryCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Commits(args) => args.run(invocation).await,
            Self::Branches(args) => args.run(invocation).await,
            Self::Releases(args) => args.run(invocation).await,
        }
    }
}
