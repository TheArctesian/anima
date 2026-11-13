//! `anima event watch`: one line for each event the hub adds, until interrupted (K1
//! `eventAdded`). The worked example of a `watch` command: open a [`Subscription`], print each
//! answer with [`stream::line`].

use super::documents::{EventAdded, event_added};
use super::list::SeverityArg;
use crate::failure::Failure;
use crate::hub::subscription::Subscription;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};
use crate::output::text::{clean, enum_name};

#[derive(Debug, clap::Args)]
pub struct WatchArgs {
    /// Only events at or above this severity (the hub's default is info).
    #[arg(long, value_enum)]
    min_severity: Option<SeverityArg>,
}

impl WatchArgs {
    /// Prints each new event as a line for a person or, with `--json`, as one K1 `Event`
    /// object per line. Ends when the hub completes the subscription, the pipe closes or an
    /// error ends it.
    ///
    /// # Errors
    /// [`Failure`] when the hub refuses the token or the connection is lost.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = event_added::Variables {
            min_severity: self.min_severity.map(SeverityArg::watch),
        };
        let mut events = Subscription::open::<EventAdded>(&hub, variables).await?;
        while let Some(answer) = events.next().await {
            let event = answer?.event_added;
            let text = if invocation.output.json() {
                serde_json::to_string(&event).map_err(|error| Failure::Write {
                    reason: error.to_string(),
                })?
            } else {
                line(&event)
            };
            if stream::line(&text)? == Reader::Gone {
                break;
            }
        }
        Ok(())
    }
}

/// An event as one line: time, severity, source, device and message.
fn line(event: &event_added::EventAddedEventAdded) -> String {
    clean(&format!(
        "{}  {:<7}  {:<12}  {}  {}",
        event.at,
        enum_name(&event.severity),
        enum_name(&event.source),
        event.device_name.as_deref().unwrap_or("-"),
        event.message
    ))
}

impl SeverityArg {
    const fn watch(self) -> event_added::Severity {
        match self {
            Self::Info => event_added::Severity::INFO,
            Self::Warning => event_added::Severity::WARNING,
            Self::Error => event_added::Severity::ERROR,
        }
    }
}
