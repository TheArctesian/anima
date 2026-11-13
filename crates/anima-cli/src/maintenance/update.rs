//! `anima maintenance update <id-or-name>`: changes the fields you name and keeps the rest.
//! The hub replaces a window as a whole, so this reads it, lays the flags over it and sends it.

use super::documents::{MaintenanceUpdate, maintenance_update};
use super::fields::{INPUT, WindowFields, into_input};
use super::list::find;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    /// The window's ID or exact name.
    window: String,
    #[command(flatten)]
    fields: WindowFields,
}

impl UpdateArgs {
    /// Prints the changed window.
    ///
    /// # Errors
    /// [`Failure::Usage`] when no flag is given; [`Failure::NotFound`] for an unknown window;
    /// the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if self.fields.is_empty() {
            return Err(Failure::Usage {
                message: "Nothing to change: give at least one field to update.".to_owned(),
                hint: Some(Hint::run("anima maintenance update --help")),
            });
        }
        let hub = invocation.hub()?;
        let window = find(&hub, &self.window).await?;
        let id = window.id.clone();
        let existing = serde_json::to_value(&window).unwrap_or_default();
        let base = existing.as_object().map_or_default(|fields| {
            fields
                .iter()
                .filter(|(key, _)| INPUT.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        });
        let input = into_input(self.fields.apply(base)?)?;
        let answer = hub
            .run::<MaintenanceUpdate>(maintenance_update::Variables { id, input })
            .await?;
        invocation
            .output
            .data(&answer.update_maintenance_window, |window| {
                let mut table = Table::record();
                table.pair("Name", window.name.as_str());
                table.pair("ID", window.id.as_str());
                table.pair("Kind", enum_name(&window.kind));
                table
            })
    }
}
