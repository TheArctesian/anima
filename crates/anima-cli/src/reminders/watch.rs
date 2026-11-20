//! `anima reminder watch`: one line for each reminder when the hub sends it (K-E6 Section 5; K1
//! `Subscription.reminderFirings`). While it runs, the hub counts it as an open application and
//! queues no reminder to a channel.

use super::documents::{ReminderWatch, reminder_watch};
use crate::failure::Failure;
use crate::hub::subscription::Subscription;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};

/// Prints each firing as a line (the time of the send, the firing ID, the text) or, with
/// `--json`, as one K1 `ReminderFiring` object per line.
///
/// # Errors
/// [`Failure`] when the hub refuses the token or the connection is lost.
pub async fn run(invocation: &Invocation) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let mut firings = Subscription::open::<ReminderWatch>(&hub, reminder_watch::Variables).await?;
    while let Some(answer) = firings.next().await {
        let firing = answer?.reminder_firings;
        if stream::emit(invocation.output.json(), &firing, line)? == Reader::Gone {
            break;
        }
    }
    Ok(())
}

fn line(firing: &reminder_watch::ReminderWatchReminderFirings) -> String {
    let sent = firing.sent_at.as_deref().unwrap_or("-");
    format!("{sent}  {}  {}", firing.id, firing.text)
}
