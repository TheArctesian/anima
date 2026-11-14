//! `anima source remove <id-or-label>`: deletes the account and its stored key (ITG-004).

use super::account::resolve;
use super::documents::{SourceRemove, source_remove};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
#[command(group(clap::ArgGroup::new("records").required(true).args(["keep_records", "delete_records"])))]
pub struct RemoveArgs {
    /// The account's ID or its exact label.
    account: String,
    /// Keep what the account imported.
    #[arg(long)]
    keep_records: bool,
    /// Delete what the account imported.
    #[arg(long)]
    delete_records: bool,
}

impl RemoveArgs {
    /// Asks to confirm, naming the account (or takes `--yes`, CMD-024), removes it and prints
    /// its ID. Nothing is sent when the operator declines.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown account; [`Failure::Declined`] when the operator
    /// says no; [`Failure::Usage`] without a confirmation when no one can be asked; the hub's
    /// refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let named = resolve(&hub, &self.account).await?;
        let records = if self.delete_records {
            "and DELETE the records it imported"
        } else {
            "and keep the records it imported"
        };
        invocation.prompter.confirm(&format!(
            "Remove source account {} and its stored key, {records}?",
            named.label
        ))?;
        let variables = source_remove::Variables {
            id: named.id,
            delete_records: self.delete_records,
        };
        let answer = hub.run::<SourceRemove>(variables).await?;
        invocation.output.data(&answer.remove_source_account, |id| {
            let mut table = Table::record();
            table.pair("Removed", id.as_str());
            table
        })
    }
}
