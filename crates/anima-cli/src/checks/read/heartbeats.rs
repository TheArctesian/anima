//! `anima check heartbeats <id-or-name>`: a page of heartbeats, newest first (CMD-026).

use super::documents::{CheckHeartbeats, check_heartbeats};
use crate::checks::resolve::CheckRef;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

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

type Page = check_heartbeats::CheckHeartbeatsCheckHeartbeats;

impl HeartbeatsArgs {
    /// Prints the heartbeats as a table or, with `--json`, as the K1 `HeartbeatConnection`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown check; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = self.check.resolve(&hub).await?;
        let paging = Paging {
            first: if self.all { MAX_PAGE } else { self.limit },
            after: self.after,
            all: self.all,
        };
        let beats = fetch(&hub, &id, paging).await?;
        invocation.output.data(&beats, table)?;
        if beats.page_info.has_next_page {
            let cursor = beats.page_info.end_cursor.as_deref().unwrap_or_default();
            invocation.output.note(&format!(
                "{} heartbeats. Next page: anima check heartbeats {} --after {cursor} (or --all)",
                beats.edges.len(),
                self.check.label()
            ));
        }
        Ok(())
    }
}

/// Which page to ask for, and whether to follow the rest.
struct Paging {
    first: i64,
    after: Option<String>,
    all: bool,
}

async fn fetch(hub: &Hub, id: &str, paging: Paging) -> Result<Page, Failure> {
    let Paging {
        first,
        mut after,
        all,
    } = paging;
    let mut joined: Option<Page> = None;
    loop {
        let variables = check_heartbeats::Variables {
            id: id.to_owned(),
            first: Some(first),
            after: after.take(),
        };
        let answer = hub.run::<CheckHeartbeats>(variables).await?;
        let page = answer
            .check
            .ok_or_else(|| Failure::NotFound {
                message: format!("No check has the ID {id}."),
                hint: Some("anima check list --device <id-or-name>".to_owned()),
            })?
            .heartbeats;
        let next = page
            .page_info
            .end_cursor
            .clone()
            .filter(|_| page.page_info.has_next_page);
        let mut beats = match joined.take() {
            Some(mut beats) => {
                beats.edges.extend(page.edges);
                beats.page_info = page.page_info;
                beats
            }
            None => page,
        };
        match next {
            Some(cursor) if all => {
                after = Some(cursor);
                joined = Some(beats);
            }
            _ => {
                beats.page_info.has_next_page &= !all;
                return Ok(beats);
            }
        }
    }
}

fn table(beats: &Page) -> Table {
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
