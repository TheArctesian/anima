//! `anima web delete`: removes a web interface from its device.

use super::documents::{WebDelete, web_delete};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct DeleteArgs {
    /// The web interface's ID (`anima web list <device>`).
    id: String,
}

impl DeleteArgs {
    /// Asks for confirmation (`--yes` skips it), deletes and prints the deleted ID.
    ///
    /// # Errors
    /// [`Failure::Usage`] when it cannot ask; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let question = format!("Delete the web interface {}?", self.id);
        invocation.prompter.confirm(&question)?;
        let answer = hub
            .run::<WebDelete>(web_delete::Variables { id: self.id })
            .await?;
        invocation.output.data(&answer.delete_web_interface, |id| {
            let mut table = Table::record();
            table.pair("Deleted", id.as_str());
            table
        })
    }
}
