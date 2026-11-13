//! `anima device wake`: a Wake-on-LAN packet from the site's relay.

use super::documents::{DeviceWake, device_wake};
use super::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct WakeArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl WakeArgs {
    /// Prints when the packet was sent and which device sent it.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal, such as a site with
    /// no wake relay.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let device_id = resolve(&hub, &self.device).await?;
        let sent = hub
            .run::<DeviceWake>(device_wake::Variables { device_id })
            .await?
            .wake_device;
        invocation.output.data(&sent, |sent| {
            let mut table = Table::record();
            table.pair("Sent at", sent.sent_at.as_str());
            table.pair("Relay", sent.relay_name.as_str());
            table
        })
    }
}
