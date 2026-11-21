//! `anima catalogue show <id>`: one entry of the catalogue, with the names of its environment
//! variables (never a value) and its Compose file (task H38).

use super::documents::{CatalogueShow, catalogue_show};
use super::list::{NOT_TESTED, words};
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;

pub type Entry = catalogue_show::CatalogueShowCatalogueEntry;

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// The identifier of the entry, as `anima catalogue list` shows it.
    id: String,
}

/// The listed entry `id`.
///
/// # Errors
/// [`Failure::NotFound`] when the hub lists no such entry; the hub's refusal.
pub async fn fetch(hub: &Hub, id: &str) -> Result<Entry, Failure> {
    let variables = catalogue_show::Variables { id: id.to_owned() };
    hub.run::<CatalogueShow>(variables)
        .await?
        .catalogue_entry
        .ok_or_else(|| Failure::NotFound {
            message: format!("The catalogue lists no entry {id}."),
            hint: None,
        })
}

impl ShowArgs {
    /// Prints the fields of the entry, then its Compose file; with `--json`, the K1
    /// `CatalogueEntry`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an entry that is not listed; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let entry = fetch(&invocation.hub()?, &self.id).await?;
        invocation.output.data(&entry, table)?;
        if !invocation.output.json() {
            invocation.output.line("")?;
            invocation.output.file(&entry.compose_file)?;
        }
        Ok(())
    }
}

fn or_dash(list: &[String]) -> String {
    if list.is_empty() {
        "-".to_owned()
    } else {
        list.join(", ")
    }
}

/// The tier in words, with the publisher and the day of its data for a generated entry.
fn tier(entry: &Entry) -> String {
    match &entry.origin {
        Some(origin) => format!(
            "{} from {} (data of {}); not tested by Anima",
            words(&entry.tier),
            origin.publisher,
            origin.read
        ),
        None => format!("{} by Anima", words(&entry.tier)),
    }
}

/// Who read a generated entry and when (SF-255); `-` for a tested entry.
fn reviewed(entry: &Entry) -> String {
    match (&entry.origin, &entry.reviewed_by, &entry.reviewed_on) {
        (_, Some(by), Some(on)) => format!("by {by} on {on}"),
        (Some(_), _, _) => "not reviewed".to_owned(),
        (None, _, _) => "-".to_owned(),
    }
}

/// The class in words, with the reason for a shell (OD-HOST-54).
fn class(entry: &Entry) -> String {
    if matches!(entry.class, catalogue_show::CatalogueEntryClass::SHELL) {
        "shell: needs a password made on the machine; no public route; only a person in the \
         Anima app in a web browser installs it"
            .to_owned()
    } else {
        words(&entry.class)
    }
}

fn table(entry: &Entry) -> Table {
    let mut table = Table::record();
    table.pair("ID", entry.id.as_str());
    table.pair("Name", entry.name.as_str());
    table.pair("Description", entry.description.as_str());
    table.pair("Kind", words(&entry.kind));
    table.pair("Version", entry.version.as_str());
    table.pair("Author", entry.author.as_str());
    table.pair("Tags", or_dash(&entry.tags));
    table.pair("Documentation", entry.documentation_url.as_str());
    let marks: Vec<String> = entry.marks.iter().map(words).collect();
    table.pair("Marks", or_dash(&marks));
    let ports: Vec<String> = (entry.network_ports.iter())
        .map(|open| format!("{} ({})", open.port, open.protocols.join(", ")))
        .collect();
    table.pair("Ports on the network", or_dash(&ports));
    table.pair("Tier", tier(entry));
    table.pair("Reviewed", reviewed(entry));
    table.pair("Class", class(entry));
    let streams: Vec<String> = (entry.streams.iter())
        .map(|stream| format!("{} ({})", stream.port, stream.protocols.join(", ")))
        .collect();
    table.pair("Streams", or_dash(&streams));
    let optional: Vec<String> = (entry.optional_parts.iter())
        .map(|part| {
            let state = if part.on { "on" } else { "off" };
            let setting = if part.needs_machine_setting {
                ", needs a machine setting"
            } else {
                ""
            };
            format!("{} {} ({state}{setting})", words(&part.kind), part.path)
        })
        .collect();
    table.pair("Optional parts", or_dash(&optional));
    table.pair("Digests", or_dash(&entry.digests));
    let tested = entry.last_machine_test.as_deref().unwrap_or(NOT_TESTED);
    table.pair("Machine test", tested);
    table.pair("Pinned", entry.pinned.as_str());
    let install = entry.install.reason.as_deref().unwrap_or("possible");
    table.pair("Install", install);
    if entry.install.needs_open_to_network {
        table.pair("Machine setting", "Open to the network");
    }
    table.pair("Variables", or_dash(&entry.environment_variables));
    table
}
