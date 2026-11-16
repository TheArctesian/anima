//! `anima link revoke <key>`: the device leaves the link and its streams close (LNK-004).

use super::documents::{LinkRevoke, link_revoke};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct RevokeArgs {
    /// The device's key, as `anima link devices` shows it.
    key: String,
}

impl RevokeArgs {
    /// Asks to confirm (or takes `--yes`, CMD-024), revokes the device and prints how many
    /// streams closed.
    ///
    /// # Errors
    /// [`Failure::Usage`] without a confirmation when no one can be asked; the hub's refusal
    /// (`NOT_FOUND` for a key the link does not list, `FORBIDDEN` for an API key).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        invocation
            .prompter
            .confirm(&format!("Revoke the link device with key {}?", self.key))?;
        let answer = hub
            .run::<LinkRevoke>(link_revoke::Variables { key: self.key })
            .await?;
        invocation
            .output
            .data(&answer.revoke_link_device, |closed| {
                let mut table = Table::record();
                table.pair("Streams closed", closed.to_string());
                table
            })
    }
}
