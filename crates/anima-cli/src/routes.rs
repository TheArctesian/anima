//! `anima route`: the public names of applications, served by the hub (HOST-046, task H21).
//! A name serves only after its proof, the consent of the machine and its certificate.

pub mod add;
pub mod check;
pub mod documents;
pub mod list;
pub mod remove;
pub mod show;
pub mod shown;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum RouteCommand {
    /// List the public names of an application, or of each.
    List(list::ListArgs),
    /// Show one public name with its proof, certificate and counts.
    Show(show::ShowArgs),
    /// Make an application public under a name (login token only).
    Add(add::AddArgs),
    /// Read the proof of a name once (login token only).
    Check(check::CheckArgs),
    /// Remove a public name (login token only).
    Remove(remove::RemoveArgs),
}

impl RouteCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown route, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Show(args) => args.run(invocation).await,
            Self::Add(args) => args.run(invocation).await,
            Self::Check(args) => args.run(invocation).await,
            Self::Remove(args) => args.run(invocation).await,
        }
    }
}
