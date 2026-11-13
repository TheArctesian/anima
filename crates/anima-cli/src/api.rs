//! `anima api`: sends a GraphQL document the operator wrote and prints the hub's whole answer.
//! The document is not checked against K1 here; the hub checks it.

use std::fs::File;
use std::io::{self, Read as _};
use std::path::PathBuf;

use crate::failure::Failure;
use crate::invocation::Invocation;

/// The largest document or variables file read.
const MAX_INPUT: usize = 1024 * 1024;

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
        let document = String::from_utf8(read(&self.document)?).map_err(|_| Failure::Input {
            what: "the document".to_owned(),
            reason: "it is not UTF-8 text".to_owned(),
        })?;
        let variables = match &self.variables {
            Some(path) => parse_variables(&read(path)?)?,
            None => serde_json::Value::Object(serde_json::Map::new()),
        };
        let body = serde_json::json!({ "query": document, "variables": variables });
        let hub = invocation.hub()?;
        let (answer, error) = hub.send_raw(&body).await?;
        invocation.output.raw(&answer)?;
        error.map_or(Ok(()), |error| Err(error.into()))
    }
}

fn parse_variables(bytes: &[u8]) -> Result<serde_json::Value, Failure> {
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(value) if value.is_object() => Ok(value),
        _ => Err(Failure::Usage {
            message: "--variables must be a JSON object.".to_owned(),
            hint: None,
        }),
    }
}

/// The bytes of `path`, or of standard input for `-`, up to [`MAX_INPUT`].
fn read(path: &PathBuf) -> Result<Vec<u8>, Failure> {
    let failed = |reason: String| Failure::Input {
        what: path.display().to_string(),
        reason,
    };
    let mut bytes = Vec::new();
    let result = if path.as_os_str() == "-" {
        io::stdin()
            .lock()
            .take(MAX_INPUT as u64 + 1)
            .read_to_end(&mut bytes)
    } else {
        File::open(path).and_then(|file| file.take(MAX_INPUT as u64 + 1).read_to_end(&mut bytes))
    };
    result.map_err(|error| failed(error.to_string()))?;
    if bytes.len() > MAX_INPUT {
        return Err(failed(format!("it is larger than {MAX_INPUT} bytes")));
    }
    Ok(bytes)
}
