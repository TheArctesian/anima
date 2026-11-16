//! `anima code show <repository> <path>` (alias `cat`): the text of one file on standard
//! output (PRJ-040). The hub refuses an API key (D-23), so this needs a login.

use super::documents::{CodeFileRead, code_file_read};
use crate::failure::{Failure, exit};
use crate::invocation::Invocation;
use crate::output::Table;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The file from the repository root, as it is (spaces and non-ASCII letters included).
    path: String,
    /// A branch name or a 40-digit commit; default the repository's default branch.
    #[arg(long = "ref")]
    reference: Option<String>,
}

impl ShowArgs {
    /// Prints the file's text and nothing else (to a terminal with its control characters made
    /// harmless) or, with `--json`, the whole K1 `CodeFile`, whatever its `content`.
    ///
    /// # Errors
    /// [`Failure::Ended`] with exit 1 for a file that is binary or too large (the message holds
    /// its link on GitHub); the hub's refusal for an API key, a directory, a symbolic link or a
    /// submodule.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = code_file_read::Variables {
            repository: resolve(&hub, &self.repository).await?,
            ref_: self.reference,
            path: self.path,
        };
        let file = hub.run::<CodeFileRead>(variables).await?.code_file;
        if invocation.output.json() {
            // With `--json` the object is printed whole and no table is made.
            return invocation.output.data(&file, |_| Table::record());
        }
        let cannot = |why: &str| Failure::Ended {
            message: format!(
                "{} {why}, so it is not shown here. Open it on GitHub: {}",
                file.path, file.url
            ),
            status: exit::FAILURE,
        };
        match file.content {
            code_file_read::CodeFileContent::TEXT => invocation
                .output
                .file(file.text.as_deref().unwrap_or_default()),
            code_file_read::CodeFileContent::TOO_LARGE => {
                Err(cannot("is larger than 1,048,576 bytes"))
            }
            code_file_read::CodeFileContent::BINARY | code_file_read::CodeFileContent::Other(_) => {
                Err(cannot("is not text"))
            }
        }
    }
}
