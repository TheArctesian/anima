//! `anima maintenance create`: a window that silences the checks of its targets while it is on.

use super::documents::{MaintenanceCreate, maintenance_create};
use super::fields::{WindowFields, into_input, require};
use super::targets;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    #[command(flatten)]
    fields: WindowFields,
}

impl CreateArgs {
    /// Prints the new window.
    ///
    /// # Errors
    /// [`Failure::Usage`] for a missing or unreadable flag; the hub's refusal (VALIDATION
    /// for a schedule that does not fit its kind).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let mut input = self.fields.apply(serde_json::Map::new())?;
        require(&input)?;
        let hub = invocation.hub()?;
        targets::resolve(&hub, &mut input).await?;
        let input = into_input(input)?;
        let answer = hub
            .run::<MaintenanceCreate>(maintenance_create::Variables { input })
            .await?;
        invocation
            .output
            .data(&answer.create_maintenance_window, |window| {
                let mut table = Table::record();
                table.pair("Name", window.name.as_str());
                table.pair("ID", window.id.as_str());
                table.pair("Kind", enum_name(&window.kind));
                table
            })
    }
}
