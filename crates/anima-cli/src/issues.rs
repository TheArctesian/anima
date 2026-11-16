//! `anima issue`: the issues of a repository the hub reads live from GitHub, and the first
//! changes to them (PRJ-028, PRJ-030, PRJ-031, OD-E14).

pub mod comment;
pub mod create;
pub mod documents;
pub mod get;
pub mod list;
pub mod open;
pub mod render;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum IssueCommand {
    /// List the issues of a repository, newest updated first (pull requests are not in it).
    List(list::ListArgs),
    /// Show one issue: fields and body, and with `--comments` its comments.
    Get(get::GetArgs),
    /// Make an issue on GitHub.
    Create(create::CreateArgs),
    /// Add a comment to an issue or a pull request.
    Comment(comment::AddCommentArgs),
    /// Close an issue.
    Close(open::CloseArgs),
    /// Open a closed issue again.
    Reopen(open::ReopenArgs),
}

impl IssueCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or issue, a body that cannot be read, or a hub
    /// refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
            Self::Create(args) => args.run(invocation).await,
            Self::Comment(args) => args.run(invocation).await,
            Self::Close(args) => args.run(invocation).await,
            Self::Reopen(args) => args.run(invocation).await,
        }
    }
}
