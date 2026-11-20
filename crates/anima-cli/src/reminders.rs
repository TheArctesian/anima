//! `anima reminder`: the reminder rules and their firings (K-E6 Section 6; REM-001 to REM-007).
//! A rule has one trigger and can have one `unless`, each as JSON in the form of K-E6 Section 2.

pub mod documents;
pub mod firings;
pub mod rules;
mod shown;
mod watch;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ReminderCommand {
    /// List the rules.
    List,
    /// Show one rule.
    Show(rules::IdArgs),
    /// Make a rule.
    Add(rules::AddArgs),
    /// Change a rule: only what you give changes. A reminder that was due before is not sent.
    Set(rules::SetArgs),
    /// Delete a rule with its firings.
    Remove(rules::IdArgs),
    /// Set a rule on.
    Enable(rules::IdArgs),
    /// Set a rule off: it fires nothing, and a reminder that waits is not sent.
    Disable(rules::IdArgs),
    /// Send the rule one time, in the next minute; no cooldown and no limit apply.
    Test(rules::IdArgs),
    /// List the firings, newest first.
    Firings(firings::ListArgs),
    /// Snooze a reminder that was sent: it is sent again after the minutes.
    Snooze(firings::SnoozeArgs),
    /// Acknowledge a reminder that was sent: it is not sent again.
    Ack(firings::AckArgs),
    /// Print one line for each reminder when it is sent. While this runs, the hub sends no
    /// reminder to a notification channel.
    Watch,
}

impl ReminderCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal (`VALIDATION` with the fields to fix, `NOT_FOUND`, `CONFLICT`
    /// for a firing that was not sent), JSON that does not parse, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List => rules::list(invocation).await,
            Self::Show(args) => rules::show(invocation, &args.id).await,
            Self::Add(args) => args.run(invocation).await,
            Self::Set(args) => args.run(invocation).await,
            Self::Remove(args) => rules::remove(invocation, args.id).await,
            Self::Enable(args) => rules::enable(invocation, args.id, true).await,
            Self::Disable(args) => rules::enable(invocation, args.id, false).await,
            Self::Test(args) => firings::test(invocation, args.id).await,
            Self::Firings(args) => args.run(invocation).await,
            Self::Snooze(args) => args.run(invocation).await,
            Self::Ack(args) => args.run(invocation).await,
            Self::Watch => watch::run(invocation).await,
        }
    }
}
