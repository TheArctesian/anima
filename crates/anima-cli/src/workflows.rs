//! `anima workflow`: the workflows of a repository and their runs, read live from GitHub by the
//! hub, and the controls of a run (PRJ-029, PRJ-034, PRJ-035).

pub mod control;
pub mod documents;
pub mod inputs;
pub mod list;
pub mod query;
pub mod render;
pub mod run;
pub mod runs;
pub mod start;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum WorkflowCommand {
    /// List the workflows of a repository, by name.
    List(list::ListArgs),
    /// Start a workflow by hand on a branch. Needs the operator's sign-in.
    Start(start::StartArgs),
    /// List the runs of a repository, newest first.
    Runs(runs::RunsArgs),
    /// Show one run with its jobs.
    Run(run::RunArgs),
    /// Run a finished run again, or with `--failed` only its failed jobs. Needs the operator's
    /// sign-in.
    Rerun(control::RerunArgs),
    /// Cancel a queued or running run. Needs the operator's sign-in.
    Cancel(control::CancelArgs),
}

impl WorkflowCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository, workflow or run, a declined confirmation, or a
    /// hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Start(args) => args.run(invocation).await,
            Self::Runs(args) => args.run(invocation).await,
            Self::Run(args) => args.run(invocation).await,
            Self::Rerun(args) => args.run(invocation).await,
            Self::Cancel(args) => args.run(invocation).await,
        }
    }
}
