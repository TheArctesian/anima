//! `anima extraction`: the proposals of an extraction (K-E4 Section 10; VIS-001 to VIS-004).
//! Nothing of a proposal is a record until the operator confirms it.

pub mod change;
pub mod documents;
pub mod list;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ExtractionCommand {
    /// List the proposals of a record, or of every record.
    List(list::ListArgs),
    /// Store the typed fields of one extraction as a proposal.
    Propose(change::ProposeArgs),
    /// Confirm a proposal with the fields as you leave them.
    Confirm(change::ConfirmArgs),
    /// Reject a proposal: no record is made from it.
    Reject(change::RejectArgs),
}

impl ExtractionCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal (`VALIDATION`, `NOT_FOUND`, `CONFLICT` for a decided proposal),
    /// fields that are not JSON, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Propose(args) => args.run(invocation).await,
            Self::Confirm(args) => args.run(invocation).await,
            Self::Reject(args) => args.run(invocation).await,
        }
    }
}
