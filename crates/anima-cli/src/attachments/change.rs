//! `anima file remove`, `anima file archive add` and `anima file archive remove`.

use super::documents::{
    ArchiveAdd, ArchiveRemove, AttachmentRemove, archive_add, archive_remove, attachment_remove,
};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct RemoveArgs {
    /// The file's ID (see `anima file list`).
    id: String,
}

#[derive(Debug, clap::Args)]
pub struct ArchiveAddArgs {
    /// The record the import belongs to, as kind:id.
    #[arg(long)]
    owner: String,
    /// The name of the archive, such as the name of the export.
    #[arg(long)]
    name: String,
}

#[derive(Debug, clap::Args)]
pub struct ArchiveRemoveArgs {
    /// The archive's ID.
    id: String,
}

/// Prints `{ "id": … }` with `--json`, else `sentence`.
fn removed(invocation: &Invocation, id: &str, sentence: &str) -> Result<(), Failure> {
    if invocation.output.json() {
        return invocation.output.raw(&serde_json::json!({ "id": id }));
    }
    invocation.output.line(sentence)
}

impl RemoveArgs {
    /// Asks, then removes the file. It cannot be undone.
    ///
    /// # Errors
    /// [`Failure::Declined`]; the hub's refusal: `NOT_FOUND` when no file has this ID.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        invocation.prompter.confirm(&format!(
            "Remove the file {}? It cannot be undone.",
            self.id
        ))?;
        let variables = attachment_remove::Variables { id: self.id };
        let answer = hub.run::<AttachmentRemove>(variables).await?;
        let id = answer.delete_attachment;
        removed(invocation, &id, &format!("Removed the file {id}."))
    }
}

impl ArchiveAddArgs {
    /// Makes the archive and prints it.
    ///
    /// # Errors
    /// The hub's refusal: `VALIDATION` for a record or a name it does not take.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = archive_add::Variables {
            owner: self.owner,
            name: self.name,
        };
        let answer = hub.run::<ArchiveAdd>(variables).await?;
        invocation.output.data(&answer.create_archive, |archive| {
            let mut table = Table::record();
            table.pair("ID", archive.id.as_str());
            table.pair("Owner", archive.owner.as_str());
            table.pair("Name", archive.name.as_str());
            table.pair("Created", archive.created_at.as_str());
            table
        })
    }
}

impl ArchiveRemoveArgs {
    /// Asks, then deletes the archive with its files and every record that came from it.
    ///
    /// # Errors
    /// [`Failure::Declined`]; the hub's refusal: `NOT_FOUND` when there is no such archive.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let question = format!(
            "Delete the archive {}, its files and every record that came from it? It cannot be \
             undone.",
            self.id
        );
        invocation.prompter.confirm(&question)?;
        let variables = archive_remove::Variables { id: self.id };
        let answer = hub.run::<ArchiveRemove>(variables).await?;
        let id = answer.delete_archive;
        removed(
            invocation,
            &id,
            &format!("Deleted the archive {id} with its records."),
        )
    }
}
