//! `anima maintenance delete <id-or-name>`: asks first, or takes `--yes` (CMD-024).

use super::documents::{MaintenanceDelete, maintenance_delete};
use super::list::find;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct DeleteArgs {
    /// The window's ID or exact name.
    window: String,
}

impl DeleteArgs {
    /// Deletes the window and prints its ID.
    ///
    /// # Errors
    /// [`Failure::Usage`] when it cannot ask and `--yes` is not set, or the operator says no;
    /// [`Failure::NotFound`] for an unknown window; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let window = find(&hub, &self.window).await?;
        let question = format!("Delete maintenance window {}?", window.name);
        invocation.prompter.confirm(&question)?;
        let answer = hub
            .run::<MaintenanceDelete>(maintenance_delete::Variables { id: window.id })
            .await?;
        invocation.output.data(&answer, |deleted| {
            let mut table = Table::record();
            table.pair("Deleted", deleted.delete_maintenance_window.as_str());
            table
        })
    }
}
