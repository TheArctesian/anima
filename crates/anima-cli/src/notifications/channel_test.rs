//! `anima notify channel test <channel>`: sends a test message and shows what happened.

use super::channel::resolve;
use super::deliveries::result_cell;
use super::documents::{ChannelTest, channel_test};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::{clean, enum_name};

#[derive(Debug, clap::Args)]
pub struct TestArgs {
    /// The channel's ID or its exact name.
    channel: String,
}

impl TestArgs {
    /// Prints the delivery as a record or, with `--json`, as the K1 `Delivery`. A message that
    /// did not arrive is printed too, and then the command fails with exit 1.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown channel; [`Failure::NotDelivered`] when `ok` is
    /// false; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.channel).await?;
        let answer = hub
            .run::<ChannelTest>(channel_test::Variables { id })
            .await?;
        let delivery = answer.send_test_notification;
        invocation.output.data(&delivery, table)?;
        if delivery.ok {
            return Ok(());
        }
        Err(Failure::NotDelivered {
            message: not_delivered(&delivery),
        })
    }
}

/// The hub's fixed reason for the failure, and the HTTP or SMTP code when it has one.
fn not_delivered(delivery: &channel_test::ChannelTestSendTestNotification) -> String {
    let reason = delivery.reason.as_ref().map(enum_name).unwrap_or_default();
    let code = delivery
        .code
        .map(|code| format!(", code {code}"))
        .unwrap_or_default();
    format!(
        "The test message to {} was not delivered ({reason}{code}).",
        clean(&delivery.channel_name)
    )
}

fn table(delivery: &channel_test::ChannelTestSendTestNotification) -> Table {
    let mut table = Table::record();
    table.pair("Channel", delivery.channel_name.as_str());
    table.pair("At", delivery.at.as_str());
    table.pair(
        "Result",
        result_cell(delivery.ok, delivery.reason.as_ref(), delivery.code),
    );
    table
}
