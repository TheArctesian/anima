//! `anima audit list`: who changed what (K1 `audit`), newest first, in pages (CMD-026).

use super::documents::{AuditList, audit_list};
use crate::connection_page;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::paging::{CommandLine, MAX_PAGE, fetch, note_next, page_size};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Entries per page, 1 to 200.
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

impl ListArgs {
    /// Prints the entries as a table or, with `--json`, as the K1 `AuditConnection`, change
    /// records included (the hub has already replaced secret values in them).
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let hub = &hub;
        let entries = fetch(self.after, self.all, move |after| async move {
            let variables = audit_list::Variables { first, after };
            Ok(hub.run::<AuditList>(variables).await?.audit)
        })
        .await?;
        invocation.output.data(&entries, table)?;
        note_next(
            invocation,
            &entries,
            "entries",
            &CommandLine::new("anima audit list"),
        );
        Ok(())
    }
}

connection_page!(audit_list::AuditListAudit);

fn table(entries: &audit_list::AuditListAudit) -> Table {
    let mut table = Table::new(vec!["AT", "OPERATOR", "ACTION", "TARGET", "FROM"]);
    for edge in &entries.edges {
        let entry = &edge.node;
        table.row(vec![
            entry.at.as_str().into(),
            entry.operator_name.as_str().into(),
            entry.action.as_str().into(),
            format!("{} {}", entry.target_type, entry.target_id).into(),
            entry.client_addr.as_str().into(),
        ]);
    }
    table
}
