//! `anima tag`: the labels devices carry (CMD-002).

pub mod documents;
pub mod list;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum TagCommand {
    /// List every tag in use.
    List,
}

impl TagCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List => list::run(invocation).await,
        }
    }
}
