//! `anima device delete`: removes a device with its credentials, checks and history (DEV-015).

use super::documents::{DeviceDelete, device_delete};
use super::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct DeleteArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl DeleteArgs {
    /// Asks for confirmation (`--yes` skips it), deletes and prints the deleted ID.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; [`Failure::Usage`] when it cannot ask;
    /// the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.device).await?;
        let question = format!(
            "Delete the device {}? Its credentials, checks, heartbeats and metrics go too.",
            self.device
        );
        invocation.prompter.confirm(&question)?;
        let answer = hub
            .run::<DeviceDelete>(device_delete::Variables { id })
            .await?;
        invocation.output.data(&answer.delete_device, |id| {
            let mut table = Table::record();
            table.pair("Deleted", id.as_str());
            table
        })
    }
}
