//! `anima files ls <device>:<path>`: a folder's entries as the device reports them.

use super::codec::{Kind, Listing};
use super::describe;
use super::place::Remote;
use super::session::with_files;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::{Table, Tone};

#[derive(Debug, clap::Args)]
pub struct LsArgs {
    /// The folder, as `<device>:<path>` (the device by ID or exact name).
    remote: String,
}

impl LsArgs {
    /// Prints the entries as a table or, with `--json`, as K4's `{path, entries}`.
    ///
    /// # Errors
    /// [`Failure`] for a refusal or a failed tunnel.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let remote = Remote::parse(&self.remote)?;
        let path = remote.path.clone();
        let listing = with_files(invocation, &remote.device, async |files| {
            files
                .list(&path)
                .await
                .map_err(|error| describe(&error, &path))
        })
        .await?;
        invocation.output.data(&listing, table)
    }
}

fn table(listing: &Listing) -> Table {
    let mut table = Table::new(vec!["NAME", "KIND", "SIZE", "MODIFIED"]);
    for entry in &listing.entries {
        let kind = match entry.kind {
            Kind::File => ("file", Tone::Plain),
            Kind::Dir => ("dir", Tone::Good),
            Kind::Link => ("link", Tone::Muted),
        };
        table.row(vec![
            entry.name.as_str().into(),
            crate::output::Cell::toned(kind.0, kind.1),
            entry
                .size
                .map_or_else(|| "-".to_owned(), |size| size.to_string())
                .into(),
            entry.modified_at.as_deref().unwrap_or("-").into(),
        ]);
    }
    table
}
