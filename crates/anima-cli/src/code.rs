//! `anima code`: the files of a repository the hub reads live from GitHub (PRJ-039, PRJ-040).
//! A directory listing gives names; a file's text needs a login (D-23).

pub mod documents;
pub mod ls;
pub mod render;
pub mod show;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum CodeCommand {
    /// List one directory of a repository: kind, size, name.
    Ls(ls::LsArgs),
    /// Print a text file exactly as it is (needs a login, not an API key).
    #[command(visible_alias = "cat")]
    Show(show::ShowArgs),
}

impl CodeCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository, a path that is not a directory (or not a file),
    /// a file that cannot be shown, or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Ls(args) => args.run(invocation).await,
            Self::Show(args) => args.run(invocation).await,
        }
    }
}
