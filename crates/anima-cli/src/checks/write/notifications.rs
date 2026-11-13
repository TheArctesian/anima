//! `anima check notifications set <id-or-name>`: which channels a check tells, and how often it
//! repeats while Down (NOT-005). The channels are given by ID (see `anima notify channel list`).

use super::documents::{CheckNotificationsSet, check_notifications_set};
use crate::checks::resolve::CheckRef;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct NotificationsSetArgs {
    #[command(flatten)]
    check: CheckRef,
    /// A channel ID to tell; repeat for more than one.
    #[arg(long = "channel", value_name = "ID", required_unless_present = "none")]
    channels: Vec<String>,
    /// Tell no channel.
    #[arg(long, conflicts_with = "channels")]
    none: bool,
    /// Send again after each N Down heartbeats, 0 to 1000; 0 never repeats.
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(i64).range(0..=1000))]
    resend_every_beats: i64,
}

impl NotificationsSetArgs {
    /// Replaces the check's channels and prints the result.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown check; the hub's refusal (VALIDATION for an
    /// unknown channel).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let check_id = self.check.resolve(&hub).await?;
        let input = check_notifications_set::CheckNotificationsInput {
            channel_ids: self.channels,
            resend_every_beats: Some(self.resend_every_beats),
        };
        let answer = hub
            .run::<CheckNotificationsSet>(check_notifications_set::Variables { check_id, input })
            .await?;
        invocation
            .output
            .data(&answer.set_check_notifications, |notifications| {
                let mut table = Table::record();
                for channel in &notifications.channels {
                    table.pair(
                        &format!("Channel {}", channel.name),
                        format!("{} ({})", enum_name(&channel.kind), channel.id),
                    );
                }
                table.pair(
                    "Resend every",
                    match notifications.resend_every_beats {
                        0 => "never".to_owned(),
                        beats => format!("{beats} Down heartbeats"),
                    },
                );
                table
            })
    }
}
