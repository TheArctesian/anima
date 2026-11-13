//! `anima maintenance`: windows that silence the checks of their targets (CMD-002). Each flag
//! is a field of the K1 `MaintenanceWindowInput`, in kebab-case.

pub mod active;
pub mod create;
pub mod delete;
pub mod documents;
pub mod fields;
pub mod list;
pub mod targets;
pub mod update;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum MaintenanceCommand {
    /// List the maintenance windows.
    List(list::ListArgs),
    /// Create a window.
    Create(create::CreateArgs),
    /// Change a window; flags you leave out keep their value.
    Update(update::UpdateArgs),
    /// Delete a window.
    Delete(delete::DeleteArgs),
    /// Turn a manual window on.
    Start(active::StartArgs),
    /// Turn a manual window off.
    Stop(active::StopArgs),
}

impl MaintenanceCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown window or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Create(args) => args.run(invocation).await,
            Self::Update(args) => args.run(invocation).await,
            Self::Delete(args) => args.run(invocation).await,
            Self::Start(args) => args.run(invocation).await,
            Self::Stop(args) => args.run(invocation).await,
        }
    }
}
