//! `anima route remove <id>`: removes a public name with its certificate and its key.

use super::documents::{RouteRemove, route_remove};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct RemoveArgs {
    /// The route's ID (`anima route list`).
    id: String,
}

impl RemoveArgs {
    /// Asks for confirmation (`--yes` skips it), removes and prints the removed ID.
    ///
    /// # Errors
    /// [`Failure::Usage`] when it cannot ask; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        invocation
            .prompter
            .confirm(&format!("Remove the public name {}?", self.id))?;
        let removed = invocation
            .hub()?
            .run::<RouteRemove>(route_remove::Variables { id: self.id })
            .await?
            .remove_application_route;
        invocation.output.data(&removed, |id| {
            let mut table = Table::record();
            table.pair("Removed", id.as_str());
            table
        })
    }
}
