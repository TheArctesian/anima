//! `anima release next-version`: the version the release form proposes.

use super::documents::{ReleaseNextVersion, release_next_version};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct NextVersionArgs;

impl NextVersionArgs {
    /// Prints the version, or `-` before the first release; with `--json` the K1 value.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub
            .run::<ReleaseNextVersion>(release_next_version::Variables)
            .await?;
        invocation.output.data(&answer, table)
    }
}

fn table(answer: &release_next_version::ResponseData) -> Table {
    let mut table = Table::record();
    table.pair(
        "Next version",
        answer.next_release_version.as_deref().unwrap_or("-"),
    );
    table
}
