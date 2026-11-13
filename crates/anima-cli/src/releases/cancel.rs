//! `anima release cancel <job id>`: stops a build job that has not finished (BLD-010).

use super::documents::{ReleaseCancel, release_cancel};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

#[derive(Debug, clap::Args)]
pub struct CancelArgs {
    /// The build job's ID (from `anima release list --json`).
    job: String,
}

impl CancelArgs {
    /// Asks for confirmation (`--yes`), cancels the job and prints it.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal: an unknown job, or one that already ended.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        invocation
            .prompter
            .confirm(&format!("Cancel build job {}?", self.job))?;
        let answer = hub
            .run::<ReleaseCancel>(release_cancel::Variables { id: self.job })
            .await?;
        invocation.output.data(&answer.cancel_build_job, table)
    }
}

fn table(job: &release_cancel::ReleaseCancelCancelBuildJob) -> Table {
    let mut table = Table::record();
    table.pair("Job", job.id.as_str());
    table.pair("Release", job.release_id.as_str());
    table.pair("Version", job.version.as_str());
    table.pair("Target", enum_name(&job.target));
    table.pair("State", Cell::status(&enum_name(&job.state)));
    table.pair("Since", job.state_changed_at.as_str());
    table
}
