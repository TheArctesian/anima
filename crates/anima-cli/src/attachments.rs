//! `anima file`: the files of a record and the archives of an import (K-E4 Section 9;
//! FIL-001 to FIL-005). A record is `kind:id`. The bytes stream between a file of this computer
//! and the hub; they never go to a terminal.

pub mod change;
pub mod documents;
pub mod list;
pub mod transfer;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum FileCommand {
    /// List the files of a record.
    List(list::ListArgs),
    /// Send a file of this computer to the hub, sealed there.
    Upload(transfer::UploadArgs),
    /// Write the bytes of a file to a new file of this computer.
    Download(transfer::DownloadArgs),
    /// Remove a file; its stored piece goes when no other file has the same bytes.
    Remove(change::RemoveArgs),
    /// The archives of imports (add, remove).
    Archive {
        #[command(subcommand)]
        command: ArchiveCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum ArchiveCommand {
    /// Make an archive: the unit of deletion of one import.
    Add(change::ArchiveAddArgs),
    /// Delete an archive with its files and every record that came from it.
    Remove(change::ArchiveRemoveArgs),
}

impl FileCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal (`VALIDATION`, `NOT_FOUND`, `UNAVAILABLE` with no file store),
    /// a file of this computer that cannot be read or written, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Upload(args) => args.run(invocation).await,
            Self::Download(args) => args.run(invocation).await,
            Self::Remove(args) => args.run(invocation).await,
            Self::Archive { command } => match command {
                ArchiveCommand::Add(args) => args.run(invocation).await,
                ArchiveCommand::Remove(args) => args.run(invocation).await,
            },
        }
    }
}
