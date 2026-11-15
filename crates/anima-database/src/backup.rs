//! Backups of the hub's database, and a check that the cluster is not damaged.
//!
//! A backup is `pg_dump`'s custom format of the `anima` database, as `deploy/backup/backup.sh`
//! makes it on the VPS (DB-007). It holds everything the hub knows, so the file has mode 0600
//! and nothing of it is printed. A copy of the cluster folder while the server runs is not a
//! backup.
//!
//! Each function needs the server to run and signs in as the superuser through the socket.
//! None takes the lock: a backup is made while the hub serves.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{ExitStatus, Stdio};

use crate::cluster::SUPERUSER;
use crate::urls::DATABASE;
use crate::{Layout, Program, Tree, owner_only};

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("could not {step}")]
    Io {
        step: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("pg_dump ended with {status}; is the database running?")]
    Dump { status: ExitStatus },
    #[error("the backup file is not there")]
    NoFile,
    #[error(
        "pg_restore ended with {status} and changed nothing. Either the database does not \
         run, or it is not empty, or it lacks a login that the backup names, or the file is \
         not a backup of this database"
    )]
    Restore { status: ExitStatus },
    #[error("pg_amcheck ended with {status} and checked nothing; is the database running?")]
    Check { status: ExitStatus },
}

fn io_error(step: &'static str) -> impl FnOnce(io::Error) -> BackupError {
    move |source| BackupError::Io { step, source }
}

/// Writes a backup of the hub's database to `file`, with mode 0600. The file appears whole or
/// not at all: it is written beside its place and moved in.
///
/// # Errors
/// When the file cannot be written or `pg_dump` fails. Nothing `pg_dump` prints is passed
/// on: an error of it can quote a row.
pub fn write(tree: &Tree, layout: &Layout, file: &Path) -> Result<(), BackupError> {
    let mut beside = file.to_path_buf().into_os_string();
    beside.push(".new");
    let beside = Path::new(&beside);
    let new = owner_only::new_file(beside).map_err(io_error("make the backup file"))?;
    let dumped = tree
        .command(Program::PgDump)
        .arg("--host")
        .arg(layout.socket_folder())
        .args(["--username", SUPERUSER, "--format", "custom"])
        // `check` puts the `amcheck` extension into each database. It is a tool of this
        // program, not a part of the hub's database, so a backup does not carry it.
        .args(["--exclude-extension", "amcheck", DATABASE])
        .stdin(Stdio::null())
        .stdout(new.try_clone().map_err(io_error("make the backup file"))?)
        .stderr(Stdio::null())
        .status()
        .map_err(io_error("run pg_dump"));
    let finished = dumped.and_then(|status| {
        if status.success() {
            new.sync_all().map_err(io_error("write the backup file"))
        } else {
            Err(BackupError::Dump { status })
        }
    });
    if let Err(error) = finished {
        // A part of a backup is of no use and holds the hub's rows.
        fs::remove_file(beside).ok();
        return Err(error);
    }
    fs::rename(beside, file).map_err(io_error("move the backup file into place"))
}

/// Restores the backup in `file` into the hub's database, which must be empty: a new cluster
/// after its first start. All of it is restored or nothing: one object that is there already
/// ends the restore and undoes it.
///
/// # Errors
/// When the file is not there, or `pg_restore` cannot run or fails.
pub fn restore(tree: &Tree, layout: &Layout, file: &Path) -> Result<(), BackupError> {
    // A whole path, so a name that starts with `-` is never read as an option.
    let file = fs::canonicalize(file).map_err(|_| BackupError::NoFile)?;
    let status = tree
        .command(Program::PgRestore)
        .arg("--host")
        .arg(layout.socket_folder())
        .args(["--username", SUPERUSER, "--dbname", DATABASE])
        .args(["--exit-on-error", "--single-transaction"])
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(io_error("run pg_restore"))?;
    if status.success() {
        Ok(())
    } else {
        Err(BackupError::Restore { status })
    }
}

/// What a check of every table and index found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checked {
    Sound,
    /// The damaged tables and indexes, as `database.schema.name`.
    Damaged(BTreeSet<String>),
}

/// Checks every table and B-tree index of the cluster with `pg_amcheck`, for use after an
/// upgrade and after a recovery from a kill or a power cut. The first check adds the `amcheck`
/// extension to each database; it changes nothing else.
///
/// # Errors
/// When `pg_amcheck` cannot run or could not check.
pub fn check(tree: &Tree, layout: &Layout) -> Result<Checked, BackupError> {
    let output = tree
        .command(Program::PgAmcheck)
        .arg("--host")
        .arg(layout.socket_folder())
        .args(["--username", SUPERUSER, "--all", "--install-missing"])
        .stdin(Stdio::null())
        .output()
        .map_err(io_error("run pg_amcheck"))?;
    let damaged = damaged_in(&String::from_utf8_lossy(&output.stdout));
    match output.status.code() {
        Some(0) => Ok(Checked::Sound),
        // 2: damage was found. Any other code with no finding is a check that did not run.
        Some(2) if !damaged.is_empty() => Ok(Checked::Damaged(damaged)),
        _ => Err(BackupError::Check {
            status: output.status,
        }),
    }
}

/// The relations `pg_amcheck` names as damaged. Its lines start with the kind and the quoted
/// name, as `btree index "anima.checks.checks_pkey":`; the detail after it is not kept. A name
/// with a quotation mark in it would be cut there; no name of the hub has one.
fn damaged_in(report: &str) -> BTreeSet<String> {
    report
        .lines()
        .filter_map(|line| {
            let named = line
                .strip_prefix("heap table \"")
                .or_else(|| line.strip_prefix("btree index \""))?;
            Some(named.split_once('"')?.0.to_owned())
        })
        .collect()
}
