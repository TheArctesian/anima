//! `anima release publishing`: whether this hub can publish a release. A hub without the release
//! bucket and the updater key refuses `release create` (PRO-037).

use super::documents::{ReleasePublishing, release_publishing};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct PublishingArgs;

impl PublishingArgs {
    /// Prints `on` or `off`; with `--json` the K1 value, `true` or `false`.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub
            .run::<ReleasePublishing>(release_publishing::Variables)
            .await?;
        invocation
            .output
            .data(&answer.release_publishing, |on| table(*on))
    }
}

fn table(on: bool) -> Table {
    let mut table = Table::record();
    table.pair("Publishing", if on { "on" } else { "off" });
    table
}
