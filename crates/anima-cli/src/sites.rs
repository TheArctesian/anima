//! `anima site`: the networks the hub manages devices on (CMD-002).

pub mod create;
pub mod documents;
pub mod list;
pub mod set_wake_relay;
pub mod shown;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum SiteCommand {
    /// List sites.
    List,
    /// Add a site.
    Create(create::CreateArgs),
    /// Choose the device that sends Wake-on-LAN packets at a site.
    SetWakeRelay(set_wake_relay::SetWakeRelayArgs),
}

impl SiteCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown site or device, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List => list::run(invocation).await,
            Self::Create(args) => args.run(invocation).await,
            Self::SetWakeRelay(args) => args.run(invocation).await,
        }
    }
}
