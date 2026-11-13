//! `anima check heartbeats <id-or-name>`: a page of heartbeats, newest first (CMD-026).

use super::documents::{CheckHeartbeats, check_heartbeats};
use crate::checks::resolve::CheckRef;
use crate::connection_page;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};
use crate::paging::{CommandLine, fetch, note_next, page_size};

/// The hub's largest heartbeat page; `--all` asks for pages of this size.
const MAX_PAGE: i64 = 500;

#[derive(Debug, clap::Args)]
pub struct HeartbeatsArgs {
    #[command(flatten)]
    check: CheckRef,
    /// Heartbeats per page, 1 to 500.
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

impl HeartbeatsArgs {
    /// Prints the heartbeats as a table or, with `--json`, as the K1 `HeartbeatConnection`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown check; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = self.check.resolve(&hub).await?;
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let (hub, id) = (&hub, &id);
        let beats = fetch(self.after, self.all, move |after| async move {
            let variables = check_heartbeats::Variables {
                id: id.clone(),
                first,
                after,
            };
            let check = hub
                .run::<CheckHeartbeats>(variables)
                .await?
                .check
                .ok_or_else(|| Failure::NotFound {
                    message: format!("No check has the ID {id}."),
                    hint: Some(Hint::run("anima check list --device <id-or-name>")),
                })?;
            Ok(check.heartbeats)
        })
        .await?;
        invocation.output.data(&beats, table)?;
        let command = CommandLine::new("anima check heartbeats")
            .word(self.check.label())
            .flag("--device", self.check.device());
        note_next(invocation, &beats, "heartbeats", &command);
        Ok(())
    }
}

connection_page!(check_heartbeats::CheckHeartbeatsCheckHeartbeats);

fn table(beats: &check_heartbeats::CheckHeartbeatsCheckHeartbeats) -> Table {
    let mut table = Table::new(vec!["AT", "STATUS", "RESPONSE", "MESSAGE"]);
    for edge in &beats.edges {
        let beat = &edge.node;
        table.row(vec![
            beat.at.as_str().into(),
            Cell::status(&enum_name(&beat.status)),
            beat.response_time_ms
                .map_or_else(|| "-".to_owned(), |ms| format!("{ms} ms"))
                .into(),
            beat.message.as_deref().unwrap_or("").into(),
        ]);
    }
    table
}
