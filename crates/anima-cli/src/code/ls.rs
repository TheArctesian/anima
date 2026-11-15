//! `anima code ls <repository> [path]`: one directory, names only.

use super::documents::{CodeDirectoryRead, code_directory_read};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct LsArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The directory from the repository root; default the root.
    path: Option<String>,
    /// A branch name or a 40-digit commit; default the repository's default branch.
    #[arg(long = "ref")]
    reference: Option<String>,
    /// Skip the hub's cache and ask GitHub again.
    #[arg(long)]
    fresh: bool,
}

impl LsArgs {
    /// Prints the entries as a table or, with `--json`, as the K1 `CodeDirectory`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository, a path that is a file, or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = code_directory_read::Variables {
            repository: resolve(&hub, &self.repository).await?,
            ref_: self.reference,
            path: self.path,
            fresh: Some(self.fresh),
        };
        let directory = hub
            .run::<CodeDirectoryRead>(variables)
            .await?
            .code_directory;
        invocation.output.data(&directory, render::directory)?;
        if directory.partial {
            invocation.output.note(
                "The listing may be cut: GitHub lists at most 1,000 entries of one directory.",
            );
        }
        Ok(())
    }
}
