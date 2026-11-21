//! `anima compute`: model compute on the devices. Today the device policy of each node
//! (CAP-010 to CAP-012, CMD-030); the switch "takes work" is not here (USE-004, OD-C8).

pub mod documents;
pub mod set;
pub mod show;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ComputeCommand {
    /// The device policy of each node (show, set).
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum PolicyCommand {
    /// Show the device policy of each device, or of one, and whether it takes model work now.
    Show(show::ShowArgs),
    /// Change the named values of the device policy of a device.
    Set(set::SetArgs),
}

impl ComputeCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Policy {
                command: PolicyCommand::Show(args),
            } => args.run(invocation).await,
            Self::Policy {
                command: PolicyCommand::Set(args),
            } => args.run(invocation).await,
        }
    }
}
