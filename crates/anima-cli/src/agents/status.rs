//! `anima agent status`: whether the metrics agent of a device is reporting (MET-018).

use super::documents::{AgentStatus, agent_status};
use crate::devices::get::resolve;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl StatusArgs {
    /// Prints the metrics source, whether it is stale and when the last reading came in.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.device).await?;
        let device = hub
            .run::<AgentStatus>(agent_status::Variables { id: id.clone() })
            .await?
            .device
            .ok_or_else(|| Failure::NotFound {
                message: format!("No device has the ID {id}."),
                hint: Some(Hint::run("anima device list")),
            })?;
        invocation.output.data(&device, table)
    }
}

fn table(device: &agent_status::AgentStatusDevice) -> Table {
    let source = enum_name(&device.metrics_source);
    let state = match (source.as_str(), device.metrics_stale) {
        ("AGENT", true) => Cell::toned("stale: no batch in the last 30 seconds", Tone::Bad),
        ("AGENT", false) => Cell::toned("reporting", Tone::Good),
        _ => Cell::toned("no agent", Tone::Muted),
    };
    let mut table = Table::record();
    table.pair("Device", device.name.as_str());
    table.pair("Metrics source", source.as_str());
    table.pair("Agent", state);
    table.pair(
        "Last reading",
        device
            .readings
            .as_ref()
            .map_or("-", |readings| readings.at.as_str()),
    );
    table
}
