//! `anima event` and `anima audit`: what happened on the hub, and who changed what. `watch`
//! follows the hub's event stream (the crate README shows how a `watch` command is built).

pub mod audit;
pub mod documents;
pub mod list;
pub mod watch;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum EventCommand {
    /// List events, newest first, with filters and pages.
    List(list::ListArgs),
    /// Print each new event until interrupted.
    Watch(watch::WatchArgs),
}

#[derive(Debug, clap::Subcommand)]
pub enum AuditCommand {
    /// List the audit trail, newest first.
    List(audit::ListArgs),
}

impl EventCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Watch(args) => args.run(invocation).await,
        }
    }
}

impl AuditCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
        }
    }
}
