//! `anima snapshot`: a commit of the working tree on a scratch ref of the forge, so a second
//! machine can build changes that are not committed (JOB-016, JOB-017). It runs git on this
//! computer and makes no hub request, so it needs no context and no sign-in.

mod forge;
mod git;
mod git_output;
mod scratch_ref;
mod secret_paths;
mod working_tree;

use std::path::Path;

use serde::Serialize;

use crate::failure::{Failure, Hint, exit};
use crate::invocation::Invocation;
use crate::output::{Cell, Table};
use git::Git;
use scratch_ref::{ScratchRef, fetch_line};

#[derive(Debug, clap::Subcommand)]
pub enum SnapshotCommand {
    /// Commit the working tree and push it to refs/anima/wip/<id>; the index, branch and stash stay.
    Push {
        #[command(flatten)]
        remote: RemoteArg,
    },
    /// List the scratch refs on the remote.
    List {
        #[command(flatten)]
        remote: RemoteArg,
    },
    /// Delete one scratch ref on the remote (it confirms; --yes skips the question).
    Delete {
        /// A ref under refs/anima/wip/, or the id after it.
        #[arg(value_name = "REF")]
        scratch: String,
        #[command(flatten)]
        remote: RemoteArg,
    },
}

#[derive(Debug, clap::Args)]
pub struct RemoteArg {
    /// The git remote of the forge.
    #[arg(long = "remote", value_name = "NAME", default_value = "origin")]
    name: String,
}

/// Why a snapshot command failed.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("This folder is not inside a git working tree.")]
    NotWorkingTree,
    #[error("git could not be started: {reason}.")]
    GitMissing { reason: String },
    #[error("git {step} failed: {reason}")]
    Git { step: String, reason: String },
    #[error("Could not make the temporary index: {reason}.")]
    Temporary { reason: String },
    #[error("{name:?} is not a remote name.")]
    BadRemote { name: String },
    #[error("This repository has no remote named {name}.")]
    UnknownRemote { name: String },
    #[error("Only scratch refs under refs/anima/wip/ can be named here, not {text:?}.")]
    NotScratchRef { text: String },
    #[error(
        "Nothing was pushed: the working tree holds files that look like secrets: {}.",
        secret_paths::listed(paths)
    )]
    SecretPaths { paths: Vec<String> },
    #[error("The remote {remote} has no scratch ref {scratch}.")]
    NoSuchRef { remote: String, scratch: String },
    #[error("The remote {remote} refused {scratch}: {reason}")]
    Rejected {
        remote: String,
        scratch: String,
        reason: String,
    },
    #[error("Could not reach the remote {remote}: {reason}")]
    Unreachable { remote: String, reason: String },
}

impl From<SnapshotError> for Failure {
    fn from(error: SnapshotError) -> Self {
        let message = error.to_string();
        match error {
            SnapshotError::NotWorkingTree => Self::Usage {
                message,
                hint: Some(Hint::say("Run it inside a git working tree.")),
            },
            SnapshotError::BadRemote { .. } => Self::Usage {
                message,
                hint: Some(Hint::run("git remote -v")),
            },
            SnapshotError::NotScratchRef { .. } => Self::Usage {
                message,
                hint: Some(Hint::run("anima snapshot list")),
            },
            SnapshotError::UnknownRemote { .. } => Self::NotFound {
                message,
                hint: Some(Hint::run("git remote -v")),
            },
            SnapshotError::NoSuchRef { .. } => Self::NotFound {
                message,
                hint: Some(Hint::run("anima snapshot list")),
            },
            SnapshotError::SecretPaths { .. } => Self::Forbidden {
                message,
                hint: Some(Hint::say(
                    "Add them to .gitignore or move them out of the working tree; no flag sends them.",
                )),
            },
            SnapshotError::Rejected { .. } => Self::Conflict { message },
            SnapshotError::Unreachable { .. } => Self::Ended {
                message,
                status: exit::UNAVAILABLE,
            },
            SnapshotError::GitMissing { .. }
            | SnapshotError::Git { .. }
            | SnapshotError::Temporary { .. } => Self::Ended {
                message,
                status: exit::FAILURE,
            },
        }
    }
}

/// A pushed snapshot, as `snapshot push --json` prints it.
#[derive(Serialize)]
struct Pushed<'a> {
    #[serde(rename = "ref")]
    scratch: &'a ScratchRef,
    commit: &'a str,
    remote: &'a str,
    fetch: String,
    changed: bool,
}

/// A deleted scratch ref, as `snapshot delete --json` prints it.
#[derive(Serialize)]
struct Deleted<'a> {
    #[serde(rename = "ref")]
    scratch: &'a ScratchRef,
    commit: &'a str,
    remote: &'a str,
}

impl SnapshotCommand {
    /// Runs the command in the folder the program was started in.
    ///
    /// # Errors
    /// [`Failure`] from a [`SnapshotError`], a declined confirmation or the output.
    pub fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let folder = std::env::current_dir().map_err(|error| Failure::Input {
            what: "the current folder".to_owned(),
            reason: error.to_string(),
        })?;
        self.run_in(&folder, invocation)
    }

    fn run_in(self, folder: &Path, invocation: &Invocation) -> Result<(), Failure> {
        let git = Git::open(folder)?;
        match self {
            Self::Push { remote } => push(&git, &remote.name, invocation),
            Self::List { remote } => {
                let remote = forge::remote(&git, &remote.name)?;
                let rows = forge::list(&git, remote)?;
                invocation.output.data(&rows, |rows| {
                    let mut table = Table::new(vec!["REF", "COMMIT"]);
                    for row in rows {
                        table.row(vec![
                            Cell::new(row.scratch.as_str()),
                            Cell::new(&row.commit),
                        ]);
                    }
                    table
                })
            }
            Self::Delete { scratch, remote } => delete(&git, &scratch, &remote.name, invocation),
        }
    }
}

fn push(git: &Git, remote: &str, invocation: &Invocation) -> Result<(), Failure> {
    let remote = forge::remote(git, remote)?;
    let snapshot = working_tree::take(git)?;
    let scratch = ScratchRef::for_commit(&snapshot.commit)?;
    forge::push(git, remote, &snapshot.commit, &scratch)?;
    invocation.output.note(if snapshot.changed {
        "Pushed a snapshot of the working tree."
    } else {
        "The working tree has no changes: pushed HEAD's own commit."
    });
    let pushed = Pushed {
        scratch: &scratch,
        commit: &snapshot.commit,
        remote,
        fetch: fetch_line(remote, &scratch),
        changed: snapshot.changed,
    };
    invocation.output.data(&pushed, |pushed| {
        let mut table = Table::record();
        table.pair("Ref", pushed.scratch.as_str());
        table.pair("Commit", pushed.commit);
        table.pair("Remote", pushed.remote);
        table.pair("Fetch with", pushed.fetch.as_str());
        table
    })
}

fn delete(git: &Git, text: &str, remote: &str, invocation: &Invocation) -> Result<(), Failure> {
    let scratch = ScratchRef::parse(text)?;
    let remote = forge::remote(git, remote)?;
    let found = forge::find(git, remote, &scratch)?;
    let question = format!(
        "Delete the scratch ref {} ({}) on {remote}?",
        scratch.as_str(),
        found.commit
    );
    invocation.prompter.confirm(&question)?;
    forge::delete(git, remote, &scratch)?;
    let deleted = Deleted {
        scratch: &scratch,
        commit: &found.commit,
        remote,
    };
    invocation.output.data(&deleted, |deleted| {
        let mut table = Table::record();
        table.pair("Deleted", deleted.scratch.as_str());
        table
    })
}
