//! `anima session`, `anima ssh` and `anima files`: remote sessions through the hub (K4,
//! CMD-002). `session list|end` work with any token; `ssh` and `files` need the login token.

pub mod documents;
pub mod end;
pub mod files;
pub mod list;
pub mod pace;
pub mod start;
pub mod terminal;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum SessionCommand {
    /// List sessions, newest first, with pages.
    List(list::ListArgs),
    /// End a live session.
    End(end::EndArgs),
}

impl SessionCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::End(args) => args.run(invocation).await,
        }
    }
}
