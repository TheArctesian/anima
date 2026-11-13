//! `anima notify channel list`: what K1 returns about each channel, and nothing more.

use super::documents::{ChannelList, channel_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};

#[derive(Debug, clap::Args)]
pub struct ListArgs;

type Channel = channel_list::ChannelListNotificationChannels;
type Settings = channel_list::ChannelListNotificationChannelsSettings;

impl ListArgs {
    /// Prints the channels as a table or, with `--json`, as the K1 `NotificationChannel`
    /// list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub.run::<ChannelList>(channel_list::Variables).await?;
        invocation.output.data(&answer.notification_channels, table)
    }
}

fn table(channels: &Vec<Channel>) -> Table {
    let mut table = Table::new(vec![
        "NAME",
        "KIND",
        "TARGET",
        "DEFAULT",
        "LAST DELIVERY",
        "ID",
    ]);
    for channel in channels {
        table.row(vec![
            channel.name.as_str().into(),
            enum_name(&channel.kind).into(),
            target(&channel.settings).into(),
            if channel.on_by_default { "yes" } else { "no" }.into(),
            last_delivery(channel),
            channel.id.as_str().into(),
        ]);
    }
    table
}

/// Where the channel sends: the part of its settings that is not a secret.
fn target(settings: &Settings) -> String {
    match settings {
        Settings::WebhookSettings(webhook) => webhook.origin.clone(),
        Settings::NtfySettings(ntfy) => ntfy.server.clone(),
        Settings::EmailSettings(email) => format!("{}:{}", email.host, email.port),
        Settings::AppriseSettings(apprise) => apprise.server.clone(),
    }
}

fn last_delivery(channel: &Channel) -> Cell {
    match &channel.last_delivery {
        None => Cell::toned("never", Tone::Muted),
        Some(delivery) if delivery.ok => Cell::toned(&format!("ok {}", delivery.at), Tone::Good),
        Some(delivery) => {
            let reason = delivery.reason.as_ref().map_or_default(enum_name);
            Cell::toned(&format!("failed {reason} {}", delivery.at), Tone::Bad)
        }
    }
}
