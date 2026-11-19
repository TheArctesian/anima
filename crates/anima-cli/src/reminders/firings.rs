//! `anima reminder test|firings|snooze|ack` (REM-007).

use serde::Serialize;
use serde_json::json;

use super::documents::{
    ReminderAck, ReminderFirings, ReminderSnooze, ReminderTest, reminder_ack, reminder_firings,
    reminder_snooze, reminder_test,
};
use super::shown;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum State {
    Pending,
    Sent,
    Suppressed,
    Snoozed,
    Acknowledged,
}

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only the firings of this rule.
    #[arg(long)]
    rule: Option<String>,
    /// Only the firings in this state.
    #[arg(long, value_enum)]
    state: Option<State>,
    /// The most firings, 1 to 200.
    #[arg(long, default_value_t = 200, value_parser = clap::value_parser!(i64).range(1..=200))]
    first: i64,
}

#[derive(Debug, clap::Args)]
pub struct SnoozeArgs {
    /// The firing's ID (see `anima reminder firings`).
    firing: String,
    /// Minutes until it is sent again, 1 to 10080.
    #[arg(value_parser = clap::value_parser!(i64).range(1..=10_080))]
    minutes: i64,
}

#[derive(Debug, clap::Args)]
pub struct AckArgs {
    /// The firing's ID.
    firing: String,
}

fn show_one<T: Serialize>(invocation: &Invocation, firing: &T) -> Result<(), Failure> {
    invocation
        .output
        .data(firing, |firing| shown::firings(&json!([firing])))
}

impl ListArgs {
    /// Prints the firings as a table or, with `--json`, as the K1 `ReminderFiring` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        use reminder_firings::ReminderFiringState as K1;
        let hub = invocation.hub()?;
        let variables = reminder_firings::Variables {
            rule: self.rule,
            state: self.state.map(|state| match state {
                State::Pending => K1::PENDING,
                State::Sent => K1::SENT,
                State::Suppressed => K1::SUPPRESSED,
                State::Snoozed => K1::SNOOZED,
                State::Acknowledged => K1::ACKNOWLEDGED,
            }),
            first: Some(self.first),
        };
        let answer = hub.run::<ReminderFirings>(variables).await?;
        invocation.output.data(&answer.reminder_firings, |firings| {
            shown::firings(&serde_json::to_value(firings).unwrap_or_default())
        })
    }
}

/// `anima reminder test <id>`: one firing that the hub sends in the next minute.
///
/// # Errors
/// [`Failure`] for a hub refusal.
pub async fn test(invocation: &Invocation, id: String) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let answer = hub
        .run::<ReminderTest>(reminder_test::Variables { id })
        .await?;
    invocation
        .output
        .note("The hub sends the test in the next minute, to the channels of the rule.");
    show_one(invocation, &answer.test_reminder_rule)
}

impl SnoozeArgs {
    /// Snoozes the firing and prints it.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (`CONFLICT` for a firing that was not sent).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = reminder_snooze::Variables {
            firing: self.firing,
            minutes: self.minutes,
        };
        let answer = hub.run::<ReminderSnooze>(variables).await?;
        show_one(invocation, &answer.snooze_reminder)
    }
}

impl AckArgs {
    /// Acknowledges the firing and prints it.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (`CONFLICT` for a firing that was not sent).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = reminder_ack::Variables {
            firing: self.firing,
        };
        let answer = hub.run::<ReminderAck>(variables).await?;
        show_one(invocation, &answer.acknowledge_reminder)
    }
}
