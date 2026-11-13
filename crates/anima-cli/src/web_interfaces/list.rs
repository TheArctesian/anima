//! `anima web list`: a device's web interfaces. Suggestions wait for `web suggestion`.

use super::documents::{WebList, web_list};
use super::shown::Shown;
use crate::devices::get::resolve;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl ListArgs {
    /// Prints the web interfaces as a table or, with `--json`, the device with its interfaces
    /// and suggestions.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.device).await?;
        let device = hub
            .run::<WebList>(web_list::Variables { id: id.clone() })
            .await?
            .device
            .ok_or_else(|| Failure::NotFound {
                message: format!("No device has the ID {id}."),
                hint: Some(Hint::run("anima device list")),
            })?;
        invocation.output.data(&device, table)?;
        for suggestion in &device.web_interface_suggestions {
            invocation.output.note(&format!(
                "Suggested: {} {} (anima web suggestion accept {})",
                suggestion.label, suggestion.url, suggestion.check_id
            ));
        }
        Ok(())
    }
}

fn table(device: &web_list::WebListDevice) -> Table {
    let mut table = Table::new(vec!["ID", "LABEL", "URL", "OPENS IN", "PINNED"]);
    for web in &device.web_interfaces {
        let row = Shown {
            id: &web.id,
            label: &web.label,
            url: &web.url,
            open_in: &enum_name(&web.open_in),
            pinned_fingerprint: web.pinned_fingerprint_sha256.as_deref(),
        };
        table.row(row.row());
    }
    table
}
