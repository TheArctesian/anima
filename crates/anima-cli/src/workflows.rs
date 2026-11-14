//! `anima workflow`: the workflows of a repository and their runs, read live from GitHub by the
//! hub (PRJ-034, PRJ-035).

pub mod documents;
pub mod list;
pub mod query;
pub mod render;
pub mod run;
pub mod runs;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum WorkflowCommand {
    /// List the workflows of a repository, by name.
    List(list::ListArgs),
    /// List the runs of a repository, newest first.
    Runs(runs::RunsArgs),
    /// Show one run with its jobs.
    Run(run::RunArgs),
}

impl WorkflowCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or run, or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Runs(args) => args.run(invocation).await,
            Self::Run(args) => args.run(invocation).await,
        }
    }
}
