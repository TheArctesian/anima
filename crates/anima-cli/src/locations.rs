//! `anima location`: the samples of the position of the operator's phone, the ingest tokens
//! of the OwnTracks path, and the context of a transfer event (K-E4 Sections 11 to 13; R-145).
//! The newest sample is not the position of a past event.

pub mod context;
pub mod documents;
pub mod import;
pub mod list;
pub mod tokens;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum LocationCommand {
    /// List the samples observed in a span of time, oldest first.
    List(list::ListArgs),
    /// Send the samples of a file to the hub; safe to repeat.
    Import(import::ImportArgs),
    /// The ingest tokens of the OwnTracks app (add, list, revoke).
    Token {
        #[command(subcommand)]
        command: tokens::TokenCommand,
    },
    /// Show the context of a file transfer event: the devices, their places and the operator.
    Context(context::ContextArgs),
}

impl LocationCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unusable time or file, a refusal or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Import(args) => args.run(invocation).await,
            Self::Token { command } => command.run(invocation).await,
            Self::Context(args) => args.run(invocation).await,
        }
    }
}
