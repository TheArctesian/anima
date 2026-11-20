//! `anima sealed-record`: the bytes of a Class A record that a client sealed with the vault key
//! (K-E5 Section 3; VLT-001). The commands move opaque bytes for a test and for a backup; they
//! cannot open a record, and no command takes the vault key.

pub mod documents;

use std::path::PathBuf;

use base64::{Engine as _, engine::general_purpose::STANDARD};

use self::documents::{
    SealedRecordRemove, SealedRecordShow, SealedRecordStore, sealed_record_remove,
    sealed_record_show, sealed_record_store,
};
use crate::failure::{Failure, Hint};
use crate::input::{self, MAX_DOCUMENT};
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Subcommand)]
pub enum SealedRecordCommand {
    /// Show the sealed record of a record: its size and times; with --json the bytes as base64,
    /// or with --to-file the sealed bytes in a file.
    Show(ShowArgs),
    /// Store sealed bytes for a record; bytes stored before for it are replaced.
    Store(StoreArgs),
    /// Delete the sealed record of a record.
    Remove(OwnerArgs),
}

#[derive(Debug, clap::Args)]
pub struct OwnerArgs {
    /// The record that the bytes belong to, as kind:id.
    owner: String,
}

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// The record that the bytes belong to, as kind:id.
    owner: String,
    /// Write the sealed bytes to this file (it must not exist).
    #[arg(long)]
    to_file: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct StoreArgs {
    /// The record that the bytes belong to, as kind:id.
    owner: String,
    /// The key of the extension that owns the record.
    #[arg(long)]
    extension: String,
    /// The file of the sealed bytes, or - for standard input; at most 1 MiB.
    #[arg(long)]
    from_file: PathBuf,
    /// The file of the sealed search index, at most 1 MiB.
    #[arg(long)]
    index_file: Option<PathBuf>,
    /// The SHA-256 of the vault key as base64 (32 bytes). It is not the key.
    #[arg(long)]
    key_id: String,
}

impl SealedRecordCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal (`VALIDATION` with the fields to fix), a file that cannot be
    /// read or written, no record, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Show(args) => args.run(invocation).await,
            Self::Store(args) => args.run(invocation).await,
            Self::Remove(args) => remove(invocation, args.owner).await,
        }
    }
}

fn not_found(owner: &str) -> Failure {
    Failure::NotFound {
        message: format!("The record {owner} has no sealed record."),
        hint: None,
    }
}

impl ShowArgs {
    async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = sealed_record_show::Variables {
            owner: self.owner.clone(),
        };
        let answer = hub.run::<SealedRecordShow>(variables).await?;
        let record = answer.sealed_record.ok_or_else(|| not_found(&self.owner))?;
        if let Some(path) = &self.to_file {
            let bytes = STANDARD
                .decode(&record.sealed)
                .map_err(|_| Failure::Input {
                    what: "the sealed bytes of the hub".to_owned(),
                    reason: "they are not base64".to_owned(),
                })?;
            write_new(path, &bytes)?;
        }
        invocation.output.data(&record, |record| {
            let mut table = Table::record();
            table.pair("ID", record.id.as_str());
            table.pair("Owner", record.owner.as_str());
            table.pair("Extension", record.extension.as_str());
            table.pair(
                "Sealed bytes",
                base64_len(&record.sealed).to_string().as_str(),
            );
            let index = record
                .sealed_index
                .as_deref()
                .map_or_else(|| "-".to_owned(), |index| base64_len(index).to_string());
            table.pair("Index bytes", index.as_str());
            table.pair("Key ID", record.key_id.as_str());
            table.pair("Updated", record.updated_at.as_str());
            table
        })
    }
}

/// The count of the bytes that `text`, base64 with padding, holds.
fn base64_len(text: &str) -> usize {
    let padding = text.bytes().rev().take_while(|byte| *byte == b'=').count();
    (text.len() / 4 * 3).saturating_sub(padding)
}

fn write_new(path: &std::path::Path, bytes: &[u8]) -> Result<(), Failure> {
    use std::io::Write as _;
    let failed = |error: std::io::Error| Failure::Write {
        reason: format!("{}: {}", path.display(), error.kind()),
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(bytes).map_err(failed)
}

impl StoreArgs {
    async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let sealed = input::read(
            invocation.prompter,
            &self.from_file,
            "the sealed bytes",
            MAX_DOCUMENT,
        )?;
        let index = self
            .index_file
            .as_deref()
            .map(|path| input::read(invocation.prompter, path, "the sealed index", MAX_DOCUMENT))
            .transpose()?;
        let hub = invocation.hub()?;
        let input = sealed_record_store::SealedRecordInput {
            owner: self.owner,
            extension: self.extension,
            sealed: STANDARD.encode(&*sealed),
            sealed_index: index.map(|index| STANDARD.encode(&*index)),
            key_id: self.key_id,
        };
        let variables = sealed_record_store::Variables { input };
        let answer = hub.run::<SealedRecordStore>(variables).await?;
        invocation
            .output
            .data(&answer.store_sealed_record, |record| {
                let mut table = Table::record();
                table.pair("ID", record.id.as_str());
                table.pair("Owner", record.owner.as_str());
                table.pair(
                    "Sealed bytes",
                    base64_len(&record.sealed).to_string().as_str(),
                );
                table.pair("Updated", record.updated_at.as_str());
                table
            })
    }
}

async fn remove(invocation: &Invocation, owner: String) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    invocation
        .prompter
        .confirm(&format!("Delete the sealed record of {owner}?"))?;
    let variables = sealed_record_remove::Variables {
        owner: owner.clone(),
    };
    let answer = hub.run::<SealedRecordRemove>(variables).await?;
    if !answer.delete_sealed_record {
        return Err(Failure::NotFound {
            message: format!("The record {owner} has no sealed record."),
            hint: Some(Hint::run("anima sealed-record show <owner>")),
        });
    }
    invocation.output.data(&owner, |owner| {
        let mut table = Table::record();
        table.pair("Deleted", owner.as_str());
        table
    })
}
