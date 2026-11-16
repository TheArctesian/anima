//! `anima record-link add` and `anima record-link remove`.

use super::documents::{RecordLinkAdd, RecordLinkRemove, record_link_add, record_link_remove};
use super::list::shown;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    /// The record the link starts at, as kind:id.
    source: String,
    /// The record the link points to, as kind:id. It does not have to exist.
    target: String,
    /// What the link means, in lower snake case, such as `lent_to`.
    #[arg(long)]
    kind: String,
    /// The text to show for the target while it has no record, such as a person's name.
    #[arg(long)]
    label: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct RemoveArgs {
    /// The link's ID (see `anima record-link list`).
    id: String,
}

impl AddArgs {
    /// Makes the link and prints it. The same link again changes nothing; a new label replaces
    /// the old one.
    ///
    /// # Errors
    /// The hub's refusal: `VALIDATION` for text that is not a record, a kind or a label.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let input = record_link_add::RecordLinkInput {
            source: self.source,
            target: self.target,
            kind: self.kind,
            label: self.label,
        };
        let variables = record_link_add::Variables { input };
        let answer = hub.run::<RecordLinkAdd>(variables).await?;
        invocation.output.data(&answer.link_records, record)
    }
}

fn record(link: &record_link_add::RecordLinkAddLinkRecords) -> Table {
    let mut table = Table::record();
    table.pair("ID", link.id.as_str());
    table.pair(
        "Source",
        shown(&link.source, link.source_title.as_deref(), None).as_str(),
    );
    table.pair("Kind", link.kind.as_str());
    table.pair(
        "Target",
        shown(
            &link.target,
            link.target_title.as_deref(),
            link.label.as_deref(),
        )
        .as_str(),
    );
    table.pair("Created", link.created_at.as_str());
    table
}

impl RemoveArgs {
    /// Removes the link and says so. The two records are not changed, so it asks no
    /// confirmation.
    ///
    /// # Errors
    /// The hub's refusal: `NOT_FOUND` when no link has this ID.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = record_link_remove::Variables { id: self.id };
        let answer = hub.run::<RecordLinkRemove>(variables).await?;
        if invocation.output.json() {
            return invocation
                .output
                .raw(&serde_json::json!({ "id": answer.unlink_records }));
        }
        invocation
            .output
            .line(&format!("Removed the link {}.", answer.unlink_records))
    }
}
