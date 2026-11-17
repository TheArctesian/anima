//! `anima timeline`: what happened, from the records of every domain (TML-001 to TML-007).

pub mod documents;
pub mod list;
pub mod window;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum TimelineCommand {
    /// List what happened in a span of time, newest first.
    List(list::ListArgs),
}

impl TimelineCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unusable time, a refusal or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
        }
    }
}
