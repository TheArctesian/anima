//! `anima release`: releases of the desktop application and their build jobs (CMD-002).

pub mod cancel;
pub mod create;
pub mod documents;
pub mod follow;
pub mod list;
pub mod log;
pub mod next_version;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ReleaseCommand {
    /// List releases, newest first, with their jobs.
    List(list::ListArgs),
    /// Show the version the next release should have.
    NextVersion(next_version::NextVersionArgs),
    /// Start a release: one build job for each target.
    Create(create::CreateArgs),
    /// Cancel a build job that has not finished.
    Cancel(cancel::CancelArgs),
    /// Print the log of a build job.
    Log(log::LogArgs),
}

impl ReleaseCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown job or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::NextVersion(args) => args.run(invocation).await,
            Self::Create(args) => args.run(invocation).await,
            Self::Cancel(args) => args.run(invocation).await,
            Self::Log(args) => args.run(invocation).await,
        }
    }
}
