//! `anima extraction propose`, `confirm` and `reject`. The fields are a JSON list, as K1
//! `ExtractedFieldInput` writes them: `[{"name":"title","type":"TEXT","value":"Emma",
//! "confidence":0.8}]`; a confidence left out or null is unknown, not certain (VIS-004).

use serde::de::DeserializeOwned;

use super::documents::{
    ExtractionConfirm, ExtractionPropose, ExtractionReject, extraction_confirm, extraction_propose,
    extraction_reject,
};
use super::list::proposals_table;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Origin {
    /// The extraction ran on this computer or another client (VIS-002).
    Client,
    /// The extraction ran on the hub's side; refused for a Class A form.
    Hub,
}

#[derive(Debug, clap::Args)]
pub struct ProposeArgs {
    /// The record the proposal is for, as kind:id.
    #[arg(long)]
    owner: String,
    /// What the extension asked for, in lower snake case, such as book.
    #[arg(long)]
    form: String,
    /// The key of the extension that asked for the form, such as library.
    #[arg(long)]
    extension: String,
    /// Where the extraction ran.
    #[arg(long, value_enum)]
    origin: Origin,
    /// The file it was read from (see `anima file list`).
    #[arg(long)]
    attachment: Option<String>,
    /// The typed fields, as a JSON list.
    #[arg(long)]
    fields: String,
}

#[derive(Debug, clap::Args)]
pub struct ConfirmArgs {
    /// The proposal's ID (see `anima extraction list`).
    id: String,
    /// The fields as you leave them, as a JSON list; they are what the record is made from.
    #[arg(long)]
    fields: String,
}

#[derive(Debug, clap::Args)]
pub struct RejectArgs {
    /// The proposal's ID.
    id: String,
}

/// The fields of `--fields`. A refusal names the flag, never the text.
///
/// # Errors
/// [`Failure::Usage`] when the text is not a JSON list of fields.
pub fn parsed_fields<T: DeserializeOwned>(text: &str) -> Result<Vec<T>, Failure> {
    serde_json::from_str(text).map_err(|_| Failure::Usage {
        message: "--fields is not a JSON list of fields with name, type, value and confidence."
            .to_owned(),
        hint: None,
    })
}

impl ProposeArgs {
    /// Stores the proposal and prints it.
    ///
    /// # Errors
    /// [`Failure::Usage`] for fields that are not JSON; the hub's refusal (`VALIDATION`).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let fields = parsed_fields(&self.fields)?;
        let hub = invocation.hub()?;
        let input = extraction_propose::ProposeExtractionInput {
            owner: self.owner,
            form: self.form,
            extension: self.extension,
            origin: match self.origin {
                Origin::Client => extraction_propose::ExtractionOrigin::CLIENT,
                Origin::Hub => extraction_propose::ExtractionOrigin::HUB,
            },
            attachment_id: self.attachment,
            fields,
        };
        let variables = extraction_propose::Variables { input };
        let answer = hub.run::<ExtractionPropose>(variables).await?;
        let shown = vec![answer.propose_extraction];
        invocation
            .output
            .data(&shown[0], |_| proposals_table(&shown))
    }
}

impl ConfirmArgs {
    /// Confirms the proposal with the fields given and prints it.
    ///
    /// # Errors
    /// [`Failure::Usage`] for fields that are not JSON; the hub's refusal (`CONFLICT` for a
    /// decided proposal).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let fields = parsed_fields(&self.fields)?;
        let hub = invocation.hub()?;
        let variables = extraction_confirm::Variables {
            id: self.id,
            fields,
        };
        let answer = hub.run::<ExtractionConfirm>(variables).await?;
        let shown = vec![answer.confirm_extraction];
        invocation
            .output
            .data(&shown[0], |_| proposals_table(&shown))
    }
}

impl RejectArgs {
    /// Rejects the proposal and prints it.
    ///
    /// # Errors
    /// The hub's refusal (`NOT_FOUND`, `CONFLICT` for a decided proposal).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = extraction_reject::Variables { id: self.id };
        let answer = hub.run::<ExtractionReject>(variables).await?;
        let shown = vec![answer.reject_extraction];
        invocation
            .output
            .data(&shown[0], |_| proposals_table(&shown))
    }
}
