//! `anima file list <record>`.

use super::documents::{AttachmentList, attachment_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The record, as kind:id, such as book:1234.
    owner: String,
}

impl ListArgs {
    /// Prints the files as a table or, with `--json`, as the K1 `Attachment` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = attachment_list::Variables { owner: self.owner };
        let answer = hub.run::<AttachmentList>(variables).await?;
        invocation.output.data(&answer.attachments, table)
    }
}

/// `latitude, longitude` when the position was kept at upload, else `-`.
#[must_use]
pub fn position(latitude: Option<f64>, longitude: Option<f64>) -> String {
    match latitude.zip(longitude) {
        Some((latitude, longitude)) => format!("{latitude:.5}, {longitude:.5}"),
        None => "-".to_owned(),
    }
}

fn table(attachments: &Vec<attachment_list::AttachmentListAttachments>) -> Table {
    let mut table = Table::new(vec!["NAME", "TYPE", "SIZE", "LOCATION", "CREATED", "ID"]);
    for attachment in attachments {
        table.row(vec![
            attachment.name.as_str().into(),
            attachment.media_type.as_str().into(),
            attachment.size.to_string().as_str().into(),
            position(attachment.latitude, attachment.longitude)
                .as_str()
                .into(),
            attachment.created_at.as_str().into(),
            attachment.id.as_str().into(),
        ]);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::position;

    #[test]
    fn fil_003_a_position_shows_only_when_it_was_kept() {
        assert_eq!(position(Some(52.5), Some(-4.25)), "52.50000, -4.25000");
        assert_eq!(position(None, None), "-");
    }
}
