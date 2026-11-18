//! `anima record-link list <record>`.

use super::documents::{RecordLinkList, record_link_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The record, as kind:id, such as person:1234 or date:2026-11-16.
    record: String,
}

/// What to show for one side of a link: the record's name when the hub has one, else the
/// link's label (RLK-002), else the reference alone.
#[must_use]
pub fn shown(reference: &str, title: Option<&str>, label: Option<&str>) -> String {
    match title.or(label) {
        Some(name) => format!("{name} ({reference})"),
        None => reference.to_owned(),
    }
}

impl ListArgs {
    /// Prints the links as a table or, with `--json`, as the K1 `RecordLink` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = record_link_list::Variables {
            record: self.record,
        };
        let answer = hub.run::<RecordLinkList>(variables).await?;
        invocation.output.data(&answer.record_links, table)
    }
}

fn table(links: &Vec<record_link_list::RecordLinkListRecordLinks>) -> Table {
    let mut table = Table::new(vec!["SOURCE", "KIND", "TARGET", "CREATED", "ID"]);
    for link in links {
        table.row(vec![
            shown(&link.source, link.source_title.as_deref(), None)
                .as_str()
                .into(),
            link.kind.as_str().into(),
            shown(
                &link.target,
                link.target_title.as_deref(),
                link.label.as_deref(),
            )
            .as_str()
            .into(),
            link.created_at.as_str().into(),
            link.id.as_str().into(),
        ]);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::shown;

    #[test]
    fn rlk_002_a_target_with_no_record_shows_its_label() {
        assert_eq!(
            shown("person:1", None, Some("Ada Lovelace")),
            "Ada Lovelace (person:1)"
        );
        assert_eq!(
            shown("date:2026-11-16", Some("2026-11-16"), Some("a day")),
            "2026-11-16 (date:2026-11-16)"
        );
        assert_eq!(shown("book:1", None, None), "book:1");
    }
}
