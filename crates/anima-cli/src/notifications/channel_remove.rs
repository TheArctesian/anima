//! `anima notify channel delete <channel>`.

use super::channel::resolve;
use super::documents::{ChannelDelete, channel_delete};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct DeleteArgs {
    /// The channel's ID or its exact name.
    channel: String,
}

impl DeleteArgs {
    /// Asks to confirm (or takes `--yes`, CMD-024), deletes the channel and prints its ID.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown channel; [`Failure::Usage`] without a
    /// confirmation when no one can be asked; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.channel).await?;
        invocation
            .prompter
            .confirm(&format!("Delete notification channel {}?", self.channel))?;
        let answer = hub
            .run::<ChannelDelete>(channel_delete::Variables { id })
            .await?;
        invocation
            .output
            .data(&answer.delete_notification_channel, |id| {
                let mut table = Table::record();
                table.pair("Deleted", id.as_str());
                table
            })
    }
}
