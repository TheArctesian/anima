//! `anima builder list`: every builder; the hub has few and does not page them.

use super::documents::{BuilderList, builder_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

#[derive(Debug, clap::Args)]
pub struct ListArgs;

impl ListArgs {
    /// Prints the builders as a table or, with `--json`, the K1 `Builder` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub.run::<BuilderList>(builder_list::Variables).await?;
        invocation.output.data(&answer.builders, table)
    }
}

fn table(builders: &Vec<builder_list::BuilderListBuilders>) -> Table {
    let mut table = Table::new(vec![
        "NAME",
        "ONLINE",
        "PLATFORM",
        "ADDRESS",
        "LAST SEEN",
        "ID",
    ]);
    for builder in builders {
        let platform = format!(
            "{} {}",
            enum_name(&builder.platform.os),
            enum_name(&builder.platform.arch)
        );
        table.row(vec![
            builder.name.as_str().into(),
            Cell::new(if builder.online { "yes" } else { "no" }),
            platform.into(),
            builder.client_addr.as_str().into(),
            builder.last_seen_at.as_deref().unwrap_or("-").into(),
            builder.id.as_str().into(),
        ]);
    }
    table
}
