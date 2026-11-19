//! `anima extraction list [--owner <record>] [--state proposed|confirmed|rejected]`.

use serde::Serialize;

use super::documents::{ExtractionList, extraction_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum State {
    Proposed,
    Confirmed,
    Rejected,
}

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The record, as kind:id; every record when not given.
    #[arg(long)]
    owner: Option<String>,
    /// Only the proposals in this state.
    #[arg(long)]
    state: Option<State>,
}

impl ListArgs {
    /// Prints the proposals as a table or, with `--json`, as the K1 `ExtractionProposal` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let state = self.state.map(|state| match state {
            State::Proposed => extraction_list::ProposalState::PROPOSED,
            State::Confirmed => extraction_list::ProposalState::CONFIRMED,
            State::Rejected => extraction_list::ProposalState::REJECTED,
        });
        let variables = extraction_list::Variables {
            owner: self.owner,
            state,
        };
        let answer = hub.run::<ExtractionList>(variables).await?;
        invocation
            .output
            .data(&answer.extraction_proposals, proposals_table)
    }
}

/// One field as a table shows it: `name = value (confidence)`, with `?` for no confidence.
#[must_use]
pub fn field_line(name: &str, value: &str, confidence: Option<f64>) -> String {
    match confidence {
        Some(confidence) => format!("{name} = {value} ({confidence:.2})"),
        None => format!("{name} = {value} (?)"),
    }
}

/// The table of proposals of any of the four documents, which give the same fields.
#[must_use]
pub fn proposals_table<T: Serialize>(proposals: &Vec<T>) -> Table {
    let mut table = Table::new(vec!["OWNER", "FORM", "STATE", "FIELDS", "ID"]);
    for proposal in proposals {
        let proposal = serde_json::to_value(proposal).unwrap_or_default();
        let fields = proposal["fields"].as_array().cloned().unwrap_or_default();
        let fields: Vec<String> = fields
            .iter()
            .map(|field| {
                field_line(
                    field["name"].as_str().unwrap_or_default(),
                    field["value"].as_str().unwrap_or_default(),
                    field["confidence"].as_f64(),
                )
            })
            .collect();
        table.row(vec![
            proposal["owner"].as_str().unwrap_or_default().into(),
            proposal["form"].as_str().unwrap_or_default().into(),
            proposal["state"].as_str().unwrap_or_default().into(),
            fields.join("; ").as_str().into(),
            proposal["id"].as_str().unwrap_or_default().into(),
        ]);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::field_line;

    #[test]
    fn vis_004_a_field_with_no_confidence_shows_it_is_unknown() {
        assert_eq!(
            field_line("title", "Emma", Some(0.8)),
            "title = Emma (0.80)"
        );
        assert_eq!(field_line("title", "Emma", None), "title = Emma (?)");
    }
}
