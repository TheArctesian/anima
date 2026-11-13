//! `anima session watch`: one line for each start and end of a session, from every client (K1
//! `sessionChanged`).

use super::documents::{SessionChanged, session_changed};
use crate::failure::Failure;
use crate::hub::subscription::Subscription;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct WatchArgs;

impl WatchArgs {
    /// Prints each change as a line (time, device, protocol, operator, started or ended,
    /// client) or, with `--json`, as one K1 `SessionChange` object per line.
    ///
    /// # Errors
    /// [`Failure`] when the hub refuses the token or the connection is lost.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let mut changes =
            Subscription::open::<SessionChanged>(&hub, session_changed::Variables).await?;
        while let Some(answer) = changes.next().await {
            let change = answer?.session_changed;
            if stream::emit(invocation.output.json(), &change, line)? == Reader::Gone {
                break;
            }
        }
        Ok(())
    }
}

fn line(change: &session_changed::SessionChangedSessionChanged) -> String {
    let session = &change.session;
    let what = match (&session.ended_at, &session.end_reason) {
        (None, _) => "started".to_owned(),
        (Some(_), Some(reason)) => format!("ended ({})", enum_name(reason)),
        (Some(_), None) => "ended".to_owned(),
    };
    let at = session.ended_at.as_ref().unwrap_or(&session.started_at);
    format!(
        "{at}  {}  {}  {}  {what}  {}",
        session.device_name,
        enum_name(&session.protocol),
        session.operator_name,
        session.client_addr
    )
}
