//! `anima source kinds`: the sources the hub can read from (ITG-001).

use super::documents::{SourceKinds, source_kinds};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct KindsArgs;

impl KindsArgs {
    /// Prints the kinds as a table or, with `--json`, as the K1 `Source` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub.run::<SourceKinds>(source_kinds::Variables).await?;
        invocation.output.data(&answer.sources, table)
    }
}

fn table(sources: &Vec<source_kinds::SourceKindsSources>) -> Table {
    let mut table = Table::new(vec![
        "KEY",
        "NAME",
        "SCOPES",
        "GIVES",
        "POLLS EVERY",
        "USED BY",
    ]);
    for source in sources {
        table.row(vec![
            source.key.as_str().into(),
            source.name.as_str().into(),
            source.scopes.join(", ").into(),
            source.gives.join(", ").into(),
            format!("{}s", source.poll_interval_seconds).into(),
            source.consumers.join(", ").into(),
        ]);
    }
    table
}
