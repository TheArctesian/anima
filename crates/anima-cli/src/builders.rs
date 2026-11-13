//! `anima builder`: the computers that build releases (CMD-002).

pub mod documents;
pub mod enrol;
pub mod list;
pub mod revoke;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum BuilderCommand {
    /// List builders and whether each is online.
    List(list::ListArgs),
    /// Enrol this computer's address as a builder; prints its token one time.
    Enrol(enrol::EnrolArgs),
    /// Delete a builder, by ID or exact name, and revoke its token.
    Revoke(revoke::RevokeArgs),
}

impl BuilderCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown builder or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Enrol(args) => args.run(invocation).await,
            Self::Revoke(args) => args.run(invocation).await,
        }
    }
}
