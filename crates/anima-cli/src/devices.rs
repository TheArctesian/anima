//! `anima device`: the devices of the hub (CMD-002). The worked example for every other noun;
//! see the crate README.

pub mod documents;
pub mod get;
pub mod list;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum DeviceCommand {
    /// List devices, with filters and pages.
    List(list::ListArgs),
    /// Show one device by ID or exact name.
    Get(get::GetArgs),
}

impl DeviceCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
        }
    }
}
