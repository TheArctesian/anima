//! `anima agent enrol`: a one-time code that turns on the metrics agent of a device (MET-012).
//! Printing the code is the command's purpose; nothing else ever prints it.

use super::documents::{AgentEnrol, agent_enrol};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct EnrolArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl EnrolArgs {
    /// Prints the enrolment code and when it expires. The code works once; a new code makes
    /// the device's earlier unused codes useless.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = agent_enrol::Variables {
            device_id: resolve(&hub, &self.device).await?,
        };
        let enrolment = hub.run::<AgentEnrol>(variables).await?.enrol_agent;
        invocation.output.data(&enrolment, |enrolment| {
            let mut table = Table::record();
            table.pair("Code", enrolment.code.as_str());
            table.pair("Expires", enrolment.expires_at.as_str());
            table
        })?;
        invocation
            .output
            .note("The code works once and is not shown again.");
        Ok(())
    }
}
