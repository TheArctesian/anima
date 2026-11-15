//! The cluster: the PostgreSQL data folder of one bundled database.
//!
//! These choices are fixed when the cluster is made and cannot change without a dump: UTF-8,
//! the built-in `C.UTF-8` locale provider (no ICU, and no index that an operating system
//! update can break), data checksums (PostgreSQL 18 turns them on, and an upgrade needs both
//! sides equal) and `scram-sha-256` for every login.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::{Layout, Lock, Password, Program, Tree, owner_only};

#[cfg(test)]
mod tests;

/// The superuser of a bundled cluster. Only this crate signs in with it.
pub const SUPERUSER: &str = "anima_super";

#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    #[error("could not {step}")]
    Io {
        step: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("initdb failed: {reason}")]
    Initdb { reason: String },
    #[error(
        "the initdb password file could not be removed and still holds the superuser \
         password (initdb failed: {initdb_failed})"
    )]
    PasswordFileLeft {
        initdb_failed: bool,
        #[source]
        source: io::Error,
    },
    #[error("the cluster's PG_VERSION file holds no version number")]
    VersionFile,
}

/// How a cluster's major version stands to the tree that would run it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// The tree can run the cluster.
    Same,
    /// The cluster needs an upgrade before this tree can run it.
    ClusterOlder { cluster: u32, tree: u32 },
    /// A newer application made or upgraded the cluster. This tree must never touch it.
    ClusterNewer { cluster: u32, tree: u32 },
}

#[must_use]
pub fn fit(cluster: u32, tree: u32) -> Fit {
    match cluster.cmp(&tree) {
        std::cmp::Ordering::Equal => Fit::Same,
        std::cmp::Ordering::Less => Fit::ClusterOlder { cluster, tree },
        std::cmp::Ordering::Greater => Fit::ClusterNewer { cluster, tree },
    }
}

/// The major version in the cluster's `PG_VERSION`, or `None` when no cluster is there yet.
///
/// # Errors
/// When the file cannot be read or holds no number.
pub fn major_version(layout: &Layout) -> Result<Option<u32>, ClusterError> {
    match fs::read_to_string(layout.cluster().join("PG_VERSION")) {
        Ok(text) => text
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| ClusterError::VersionFile),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ClusterError::Io {
            step: "read the cluster's PG_VERSION",
            source,
        }),
    }
}

/// Makes the cluster with `initdb`, with `superuser_password` for [`SUPERUSER`]. The caller
/// holds the [`Lock`]: two copies that make one cluster remove each other's half-made folder.
///
/// The password reaches `initdb` through a file with mode 0600 that is removed again, never
/// through the command line or the environment (LOG-007). The root and the URL folder are
/// made with mode 0700 first.
///
/// # Errors
/// When a folder or the password file cannot be made, or `initdb` fails.
pub fn create(
    _lock: &Lock,
    tree: &Tree,
    layout: &Layout,
    superuser_password: &Password,
) -> Result<(), ClusterError> {
    for folder in [layout.root().to_path_buf(), layout.urls_folder()] {
        owner_only::folder(&folder).map_err(|source| ClusterError::Io {
            step: "make a database folder",
            source,
        })?;
    }
    let password_file = layout.urls_folder().join("initdb.password");
    let made = owner_only::new_file(&password_file)
        .and_then(|mut file| file.write_all(superuser_password.expose().as_bytes()))
        .map_err(|source| ClusterError::Io {
            step: "write the initdb password file",
            source,
        })
        .and_then(|()| run_initdb(tree, layout, &password_file));
    // Removed whether initdb passed or failed. A file that stays holds the password, so that
    // fault is reported before any other.
    match fs::remove_file(&password_file) {
        Err(source) if source.kind() != io::ErrorKind::NotFound => {
            Err(ClusterError::PasswordFileLeft {
                initdb_failed: made.is_err(),
                source,
            })
        }
        _ => made,
    }
}

fn run_initdb(tree: &Tree, layout: &Layout, password_file: &Path) -> Result<(), ClusterError> {
    let output = tree
        .command(Program::Initdb)
        .arg("--pgdata")
        .arg(layout.cluster())
        .args(["--username", SUPERUSER])
        .args(["--auth", "scram-sha-256", "--encoding", "UTF8"])
        .args([
            "--locale-provider",
            "builtin",
            "--builtin-locale",
            "C.UTF-8",
        ])
        .args(["--locale", "C", "--data-checksums"])
        .arg("--pwfile")
        .arg(password_file)
        .output()
        .map_err(|source| ClusterError::Io {
            step: "run initdb",
            source,
        })?;
    if output.status.success() {
        return Ok(());
    }
    Err(ClusterError::Initdb {
        reason: initdb_reason(&String::from_utf8_lossy(&output.stderr)),
    })
}

/// The cause in what `initdb` printed: its own `initdb: error:` line. No other line is
/// passed on, because `initdb` feeds the password to the server in a statement, and a server
/// that fails there can print that statement (LOG-007).
fn initdb_reason(errors: &str) -> String {
    errors
        .lines()
        .find_map(|line| line.strip_prefix("initdb: error: "))
        .unwrap_or("it printed no cause")
        .to_owned()
}
