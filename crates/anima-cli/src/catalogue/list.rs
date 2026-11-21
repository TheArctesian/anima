//! `anima catalogue list`: the entries of the catalogue that the hub lists (task H38).

use super::documents::{CatalogueList, catalogue_list};
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

pub type Entry = catalogue_list::CatalogueListCatalogueEntries;

/// What the list says of an entry with no test on a machine.
pub const NOT_TESTED: &str = "not tested on a machine";

#[derive(Debug, clap::Args)]
pub struct ListArgs {}

/// Each entry that the hub lists.
///
/// # Errors
/// The hub's refusal.
pub async fn fetch(hub: &Hub) -> Result<Vec<Entry>, Failure> {
    let variables = catalogue_list::Variables {};
    Ok(hub.run::<CatalogueList>(variables).await?.catalogue_entries)
}

impl ListArgs {
    /// Prints a row for each entry or, with `--json`, the K1 `CatalogueEntry` list.
    ///
    /// # Errors
    /// The hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let found = fetch(&invocation.hub()?).await?;
        invocation.output.data(&found, table)
    }
}

/// `NETWORK_PORT` as `network port`.
pub fn words<T: serde::Serialize>(name: &T) -> String {
    enum_name(name).to_lowercase().replace('_', " ")
}

fn table(found: &Vec<Entry>) -> Table {
    let mut table = Table::new(vec![
        "ID",
        "NAME",
        "KIND",
        "TIER",
        "CLASS",
        "VERSION",
        "MARKS",
        "MACHINE TEST",
        "PINNED",
    ]);
    for entry in found {
        let marks: Vec<String> = entry.marks.iter().map(words).collect();
        let marks = if marks.is_empty() {
            "-".to_owned()
        } else {
            marks.join(", ")
        };
        table.row(vec![
            entry.id.as_str().into(),
            entry.name.as_str().into(),
            words(&entry.kind).as_str().into(),
            words(&entry.tier).as_str().into(),
            words(&entry.class).as_str().into(),
            entry.version.as_str().into(),
            marks.as_str().into(),
            entry
                .last_machine_test
                .as_deref()
                .unwrap_or(NOT_TESTED)
                .into(),
            entry.pinned.as_str().into(),
        ]);
    }
    table
}
