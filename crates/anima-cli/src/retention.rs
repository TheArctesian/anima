//! `anima retention`: how long the hub keeps heartbeats, metrics, events and deliveries
//! (HUB-005). A login token only changes it (INT-003).

pub mod documents;
pub mod list;
pub mod set;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum RetentionCommand {
    /// Show each kind of record, its days and the allowed range.
    List(list::ListArgs),
    /// Set the days the hub keeps one kind of record (a login token is needed).
    Set(set::SetArgs),
}

impl RetentionCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Set(args) => args.run(invocation).await,
        }
    }
}
