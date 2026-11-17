//! `anima daemon`: the metrics daemon that runs on a device and pushes its readings (CMD-002).

pub mod documents;
pub mod enrol;
pub mod revoke;
pub mod status;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum DaemonCommand {
    /// Show whether a device's daemon is reporting.
    Status(status::StatusArgs),
    /// Make a one-time code that enrols the daemon on a device.
    Enrol(enrol::EnrolArgs),
    /// Revoke a device's daemon token.
    Revoke(revoke::RevokeArgs),
}

impl DaemonCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Status(args) => args.run(invocation).await,
            Self::Enrol(args) => args.run(invocation).await,
            Self::Revoke(args) => args.run(invocation).await,
        }
    }
}
