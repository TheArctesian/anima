//! `anima notify channel test <channel>`: sends a test message and shows what happened.

use super::channel::resolve;
use super::deliveries::result_cell;
use super::documents::{ChannelTest, channel_test};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct TestArgs {
    /// The channel's ID or its exact name.
    channel: String,
}

impl TestArgs {
    /// Prints the delivery as a record or, with `--json`, as the K1 `Delivery`. A message
    /// that did not arrive is a result, not an error: read `ok` or the RESULT line.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown channel; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.channel).await?;
        let answer = hub
            .run::<ChannelTest>(channel_test::Variables { id })
            .await?;
        invocation
            .output
            .data(&answer.send_test_notification, table)
    }
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
