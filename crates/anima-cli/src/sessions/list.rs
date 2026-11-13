//! `anima session list`: sessions newest first, with a device filter and pages (CMD-026).

use super::documents::{SessionList, session_list};
use crate::connection_page;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};
use crate::paging::{CommandLine, MAX_PAGE, fetch, note_next, page_size};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only sessions on this device, by ID or exact name.
    #[arg(long)]
    device: Option<String>,
    /// Only sessions that have not ended.
    #[arg(long)]
    live: bool,
    /// Sessions per page, 1 to 200.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

impl ListArgs {
    /// Prints the sessions as a table or, with `--json`, as the K1 `SessionConnection`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown device or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command = CommandLine::new("anima session list", self.limit)
            .flag("--device", self.device.as_deref())
            .switch("--live", self.live);
        let hub = invocation.hub()?;
        let device_id = match &self.device {
            Some(device) => Some(resolve(&hub, device).await?),
            None => None,
        };
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let live = self.live.then_some(true);
        let (hub, device_id) = (&hub, &device_id);
        let sessions = fetch(self.after, self.all, move |after| async move {
            let variables = session_list::Variables {
                device_id: device_id.clone(),
                live,
                first,
                after,
            };
            Ok(hub.run::<SessionList>(variables).await?.sessions)
        })
        .await?;
        invocation.output.data(&sessions, table)?;
        note_next(invocation, &sessions, "sessions shown", &command);
        Ok(())
    }
}

connection_page!(session_list::SessionListSessions);

fn table(sessions: &session_list::SessionListSessions) -> Table {
    let mut table = Table::new(vec![
        "DEVICE", "PROTOCOL", "OPERATOR", "STARTED", "ENDED", "CLIENT", "ID",
    ]);
    for edge in &sessions.edges {
        let session = &edge.node;
        let ended = match (&session.ended_at, &session.end_reason) {
            (None, _) => Cell::toned("live", Tone::Good),
            (Some(at), Some(reason)) => format!("{at} {}", enum_name(reason)).into(),
            (Some(at), None) => at.as_str().into(),
        };
        table.row(vec![
            session.device_name.as_str().into(),
            enum_name(&session.protocol).into(),
            session.operator_name.as_str().into(),
            session.started_at.as_str().into(),
            ended,
            session.client_addr.as_str().into(),
            session.id.as_str().into(),
        ]);
    }
    table
}
