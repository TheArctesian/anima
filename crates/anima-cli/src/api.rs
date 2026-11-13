//! `anima api`: sends a GraphQL document the operator wrote and prints the hub's whole answer.
//! The document is not checked against K1 here; the hub checks it.

use std::path::PathBuf;

use crate::failure::Failure;
use crate::input::{self, MAX_DOCUMENT};
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct ApiArgs {
    /// The file with the GraphQL document, or - for standard input.
    document: PathBuf,
    /// A file with the variables as a JSON object, or - for standard input.
    #[arg(long, value_name = "FILE")]
    variables: Option<PathBuf>,
}

impl ApiArgs {
    /// Sends the document and prints the answer; a GraphQL error in it sets the exit code.
    ///
    /// # Errors
    /// [`Failure`] for unreadable input, a failed request or the answer's first error.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let stdin = PathBuf::from("-");
        if self.variables.as_ref() == Some(&stdin) && self.document == stdin {
            return Err(Failure::Usage {
                message: "Only one of the document and --variables can come from standard input."
                    .to_owned(),
                hint: None,
            });
        }
        let prompter = invocation.prompter;
        let bytes = input::read(prompter, &self.document, "the document", MAX_DOCUMENT)?;
        let document = String::from_utf8(bytes.to_vec()).map_err(|_| Failure::Input {
            what: "the document".to_owned(),
            reason: "it is not UTF-8 text".to_owned(),
        })?;
        let variables = match &self.variables {
            Some(path) => {
                let bytes = input::read(prompter, path, "the variables", MAX_DOCUMENT)?;
                input::json::<serde_json::Map<String, serde_json::Value>>(
                    &bytes,
                    "--variables (a JSON object)",
                    "anima api --help",
                )?
                .into()
            }
            None => serde_json::Value::Object(serde_json::Map::new()),
        };
        let body = serde_json::json!({ "query": document, "variables": variables });
        let hub = invocation.hub()?;
        let (answer, error) = hub.send_raw(&body).await?;
        invocation.output.raw(&answer)?;
        error.map_or(Ok(()), |error| Err(error.into()))
    }
}
