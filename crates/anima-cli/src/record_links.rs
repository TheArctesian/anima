//! `anima record-link`: the links between records of different extensions (LNK-001, LNK-002,
//! LNK-004). A record is `kind:id`, such as `person:1234`; neither side has to exist.

pub mod change;
pub mod documents;
pub mod list;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum RecordLinkCommand {
    /// List the links that start or end at a record.
    List(list::ListArgs),
    /// Link one record to another.
    Add(change::AddArgs),
    /// Remove a link; the two records stay.
    Remove(change::RemoveArgs),
}

impl RecordLinkCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal (`VALIDATION` for text that is not a record, `NOT_FOUND` for
    /// an unknown link) or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Add(args) => args.run(invocation).await,
            Self::Remove(args) => args.run(invocation).await,
        }
    }
}
