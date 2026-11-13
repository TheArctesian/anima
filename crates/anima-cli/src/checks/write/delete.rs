//! `anima check delete <id-or-name>`: asks first, or takes `--yes` (CMD-024).

use super::documents::{CheckDelete, check_delete};
use crate::checks::resolve::CheckRef;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct DeleteArgs {
    #[command(flatten)]
    check: CheckRef,
}

impl DeleteArgs {
    /// Deletes the check and its heartbeats, and prints the deleted ID.
    ///
    /// # Errors
    /// [`Failure::Usage`] when it cannot ask and `--yes` is not set, or the operator says no;
    /// the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = self.check.resolve(&hub).await?;
        let question = format!("Delete check {} and its heartbeats?", self.check.label());
        invocation.prompter.confirm(&question)?;
        let answer = hub
            .run::<CheckDelete>(check_delete::Variables { id })
            .await?;
        invocation.output.data(&answer, |deleted| {
            let mut table = Table::record();
            table.pair("Deleted", deleted.delete_check.as_str());
            table
        })
    }
}
