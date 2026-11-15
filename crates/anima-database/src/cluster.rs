//! The cluster: the PostgreSQL data folder of one bundled database.
//!
//! These choices are fixed when the cluster is made and cannot change without a dump: UTF-8,
//! the built-in `C.UTF-8` locale provider (no ICU, and no index that an operating system
//! update can break), data checksums (PostgreSQL 18 turns them on, and an upgrade needs both
//! sides equal) and `scram-sha-256` for every login.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::urls::{self, UrlError};
use crate::{Layout, Lock, Password, Program, Role, Tree, owner_only};

#[cfg(test)]
mod tests;

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
    #[error("a cluster folder is there already; a new cluster is never made over it")]
    AlreadyThere,
    #[error(transparent)]
    Url(#[from] UrlError),
    #[error("pg_controldata did not say how the cluster was stopped")]
    ControlText,
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

/// Makes the cluster with `initdb` and writes the superuser's URL file. The caller holds the
/// [`Lock`]: two copies that make one cluster remove each other's half-made folder.
///
/// The cluster is made beside its place and moved in last, after the URL file is written.
/// So a crash at any point leaves either no cluster, and the next start makes one, or a
/// whole cluster with its superuser's file.
///
/// The password reaches `initdb` through a file with mode 0600 that is removed again, never
/// through the command line or the environment (LOG-007).
///
/// # Errors
/// When a cluster folder is there already, a folder or a file cannot be made, or `initdb`
/// fails.
pub fn create(
    _lock: &Lock,
    tree: &Tree,
    layout: &Layout,
    superuser_password: &Password,
) -> Result<(), ClusterError> {
    let io = |step| move |source| ClusterError::Io { step, source };
    // Before anything is written: a new superuser file beside an old cluster would lock
    // that cluster's owner out.
    if layout.cluster().exists() {
        return Err(ClusterError::AlreadyThere);
    }
    for folder in [layout.root().to_path_buf(), layout.urls_folder()] {
        owner_only::folder(&folder).map_err(io("make a database folder"))?;
    }
    let being_made = layout.cluster_being_made();
    if let Err(error) = fs::remove_dir_all(&being_made)
        && error.kind() != io::ErrorKind::NotFound
    {
        return Err(io("remove a half-made cluster")(error));
    }
    let password_file = layout.urls_folder().join("initdb.password");
    let made = owner_only::new_file(&password_file)
        .and_then(|mut file| file.write_all(superuser_password.expose().as_bytes()))
        .map_err(io("write the initdb password file"))
        .and_then(|()| run_initdb(tree, &being_made, &password_file));
    // Removed whether initdb passed or failed. A file that stays holds the password, so that
    // fault is reported before any other.
    if let Err(source) = fs::remove_file(&password_file)
        && source.kind() != io::ErrorKind::NotFound
    {
        return Err(ClusterError::PasswordFileLeft {
            initdb_failed: made.is_err(),
            source,
        });
    }
    made?;
    urls::write(layout, Role::Superuser, superuser_password)?;
    fs::rename(&being_made, layout.cluster()).map_err(io("move the new cluster into place"))
}

/// Whether the cluster's last server wrote its shutdown checkpoint. `false` after a kill or
/// a power cut: the next start recovers from the write-ahead log.
///
/// # Errors
/// When `pg_controldata` cannot run or prints no state.
pub fn was_stopped_cleanly(tree: &Tree, layout: &Layout) -> Result<bool, ClusterError> {
    let output = tree
        .command(Program::PgControldata)
        .arg(layout.cluster())
        .output()
        .map_err(|source| ClusterError::Io {
            step: "run pg_controldata",
            source,
        })?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Database cluster state:"))
        .map(|state| state.trim() == "shut down")
        .ok_or(ClusterError::ControlText)
}

fn run_initdb(tree: &Tree, cluster: &Path, password_file: &Path) -> Result<(), ClusterError> {
    let output = tree
        .command(Program::Initdb)
        .arg("--pgdata")
        .arg(cluster)
        .args(["--username", Role::Superuser.login()])
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
