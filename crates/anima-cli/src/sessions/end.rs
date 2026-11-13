//! `anima session end <id>`: ends a live session; an ended one comes back unchanged (K1).

use super::documents::{SessionEnd, session_end};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct EndArgs {
    /// The session's ID (from `anima session list`).
    id: String,
}

impl EndArgs {
    /// Ends the session and prints it, or with `--json` the K1 `Session`.
    ///
    /// # Errors
    /// [`Failure::Usage`] for an ID that is not a UUID; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if uuid::Uuid::parse_str(&self.id).is_err() {
            return Err(Failure::Usage {
                message: format!("{} is not a session ID.", self.id),
                hint: Some("anima session list --live".to_owned()),
            });
        }
        let hub = invocation.hub()?;
        let session = hub
            .run::<SessionEnd>(session_end::Variables { id: self.id })
            .await?
            .end_session;
        invocation.output.data(&session, table)
    }
}

fn table(session: &session_end::SessionEndEndSession) -> Table {
    let mut table = Table::record();
    table.pair("ID", session.id.as_str());
    table.pair("Device", session.device_name.as_str());
    table.pair("Protocol", enum_name(&session.protocol));
    table.pair("Operator", session.operator_name.as_str());
    table.pair("Client", session.client_addr.as_str());
    table.pair("Started", session.started_at.as_str());
    table.pair("Ended", session.ended_at.as_deref().unwrap_or("-"));
    let reason = session
        .end_reason
        .as_ref()
        .map_or_else(|| "-".to_owned(), enum_name);
    table.pair("Reason", reason);
    table
}
