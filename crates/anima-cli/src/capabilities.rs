//! `anima capability`: what each device can run, from the tool inventory its agent sends
//! (CAP-030, CMD-030).

pub mod documents;
pub mod hint;
pub mod list;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum CapabilityCommand {
    /// List each device's tools, build targets, architecture and system version.
    List(list::ListArgs),
}

impl CapabilityCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
        }
    }
}
