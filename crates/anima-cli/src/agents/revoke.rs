//! `anima agent revoke`: the agent's token stops working. The metrics source stays AGENT, shown
//! as stale, until the operator changes it (R-092 D-7).

use super::documents::{AgentRevoke, agent_revoke};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct RevokeArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl RevokeArgs {
    /// Asks for confirmation (`--yes` skips it), revokes and prints the device's metrics source.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; [`Failure::Usage`] when it cannot ask; the
    /// hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let device_id = resolve(&hub, &self.device).await?;
        let question = format!("Revoke the agent token of {}?", self.device);
        invocation.prompter.confirm(&question)?;
        let device = hub
            .run::<AgentRevoke>(agent_revoke::Variables { device_id })
            .await?
            .revoke_agent_token;
        invocation.output.data(&device, |device| {
            let mut table = Table::record();
            table.pair("Device", device.name.as_str());
            table.pair("Metrics source", enum_name(&device.metrics_source));
            table.pair("Stale", if device.metrics_stale { "yes" } else { "no" });
            table
        })
    }
}
