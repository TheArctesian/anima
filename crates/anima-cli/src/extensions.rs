//! `anima extension`: which extensions of the hub are on (EXT-011, EXT-012).

pub mod documents;
pub mod list;
pub mod needs;
pub mod set;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ExtensionCommand {
    /// List the extensions: key, whether each is on, what it needs and what needs it.
    List(list::ListArgs),
    /// Turn an extension on; asks first when that also turns others on.
    On(set::OnArgs),
    /// Turn an extension off; the hub refuses while another extension that is on needs it.
    Off(set::OffArgs),
}

impl ExtensionCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal (`CONFLICT` for an extension that is needed or not built), an
    /// unknown key or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::On(args) => args.run(invocation).await,
            Self::Off(args) => args.run(invocation).await,
        }
    }
}
