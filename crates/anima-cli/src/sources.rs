//! `anima source`: where the hub reads data from, and the accounts it reads with (ITG-001 to
//! ITG-004). A source account's credential is never shown, and the GitHub App key is read from
//! a private file or standard input only (CMD-025, PRJ-018).

pub mod account;
pub mod connect;
pub mod documents;
pub mod get;
pub mod kinds;
pub mod list;
pub mod pause;
pub mod remove;
pub mod render;
pub mod sync;
pub mod watch;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum SourceCommand {
    /// List the kinds of source the hub knows: scopes, what each gives and who uses it.
    Kinds(kinds::KindsArgs),
    /// List the source accounts with their health and last run.
    List(list::ListArgs),
    /// Show one account with its recent runs and their counts.
    Get(get::GetArgs),
    /// Connect an account (a login token is needed).
    Connect {
        #[command(subcommand)]
        command: connect::ConnectCommand,
    },
    /// Start a sync of an account now.
    Sync(sync::SyncArgs),
    /// Pause the syncs of an account.
    Pause(pause::PauseArgs),
    /// Resume the syncs of a paused account.
    Resume(pause::ResumeArgs),
    /// Remove an account and its stored key; keeps or deletes what it imported (a login token
    /// is needed).
    Remove(remove::RemoveArgs),
    /// Print each change of an account (or of all) until interrupted.
    Watch(watch::WatchArgs),
}

impl SourceCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown account or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Kinds(args) => args.run(invocation).await,
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
            Self::Connect { command } => command.run(invocation).await,
            Self::Sync(args) => args.run(invocation).await,
            Self::Pause(args) => args.run(invocation).await,
            Self::Resume(args) => args.run(invocation).await,
            Self::Remove(args) => args.run(invocation).await,
            Self::Watch(args) => args.run(invocation).await,
        }
    }
}
