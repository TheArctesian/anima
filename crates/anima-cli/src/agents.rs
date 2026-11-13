//! `anima agent`: the metrics agent that runs on a device and pushes its readings (CMD-002).

pub mod documents;
pub mod enrol;
pub mod revoke;
pub mod status;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum AgentCommand {
    /// Show whether a device's agent is reporting.
    Status(status::StatusArgs),
    /// Make a one-time code that enrols the agent on a device.
    Enrol(enrol::EnrolArgs),
    /// Revoke a device's agent token.
    Revoke(revoke::RevokeArgs),
}

impl AgentCommand {
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
