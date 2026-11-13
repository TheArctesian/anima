//! `anima session list`: sessions newest first, with a device filter and pages (CMD-026).

use super::documents::{SessionList, session_list};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};

/// The hub's largest page of sessions (sessions `page.rs`).
const MAX_PAGE: i64 = 200;

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
        let hub = invocation.hub()?;
        let device_id = match &self.device {
            Some(device) => Some(resolve(&hub, device).await?),
            None => None,
        };
        let variables = session_list::Variables {
            device_id,
            live: self.live.then_some(true),
            first: Some(if self.all { MAX_PAGE } else { self.limit }),
            after: self.after,
        };
        let sessions = fetch(&hub, variables, self.all).await?;
        invocation.output.data(&sessions, table)?;
        if sessions.page_info.has_next_page {
            let cursor = sessions.page_info.end_cursor.as_deref().unwrap_or_default();
            invocation.output.note(&format!(
                "{} sessions shown. Next page: anima session list --after {cursor} (or --all)",
                sessions.edges.len()
            ));
        }
        Ok(())
    }
}

/// The first page, or with `all` every page joined into one connection.
async fn fetch(
    hub: &Hub,
    mut variables: session_list::Variables,
    all: bool,
) -> Result<session_list::SessionListSessions, Failure> {
    let mut joined: Option<session_list::SessionListSessions> = None;
    loop {
        let page = hub.run::<SessionList>(variables.clone()).await?.sessions;
        let next = page
            .page_info
            .end_cursor
            .clone()
            .filter(|_| all && page.page_info.has_next_page);
        let mut sessions = match joined.take() {
            Some(mut sessions) => {
                sessions.edges.extend(page.edges);
                sessions.page_info = page.page_info;
                sessions
            }
            None => page,
        };
        let Some(cursor) = next else {
            sessions.page_info.has_next_page &= !all;
            return Ok(sessions);
        };
        variables.after = Some(cursor);
        joined = Some(sessions);
    }
}

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
