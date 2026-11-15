//! What one copy does to bring the database up: take the lock, clear away what an earlier
//! copy left, make the cluster and the roles at the first run, start the server and prove it
//! ready.

use std::process::ExitStatus;

use crate::cluster::{self, ClusterError, Fit};
use crate::roles::{self, RolesError};
use crate::urls::{self, UrlError};
use crate::{
    Layout, Limits, Lock, Platform, Role, Server, ServerError, Settings, Stopped, Tree, TreeError,
};

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error(transparent)]
    Server(#[from] ServerError),
    #[error(transparent)]
    Cluster(#[from] ClusterError),
    #[error(transparent)]
    Tree(#[from] TreeError),
    #[error(transparent)]
    Roles(#[from] RolesError),
    #[error(transparent)]
    Url(#[from] UrlError),
    #[error(
        "the database was last run by a newer Anima (PostgreSQL {cluster}); this one has \
         PostgreSQL {tree} and must not touch it. Install the newer Anima again"
    )]
    ClusterNewer { cluster: u32, tree: u32 },
    #[error(
        "the database is of PostgreSQL {cluster} and this Anima has PostgreSQL {tree}. \
         Install the Anima release that upgrades the database from PostgreSQL {cluster}"
    )]
    ClusterOlder { cluster: u32, tree: u32 },
}

impl StartError {
    /// Whether the start was refused because another copy runs this database.
    #[must_use]
    pub fn is_another_copy(&self) -> bool {
        matches!(self, Self::Server(ServerError::AnotherCopy))
    }

    /// Whether the start was refused because the cluster is of another major version. The
    /// cluster was not touched.
    #[must_use]
    pub fn is_version_refusal(&self) -> bool {
        matches!(self, Self::ClusterNewer { .. } | Self::ClusterOlder { .. })
    }
}

/// What happened to the Time Machine exclusion of a new cluster (macOS). A copy of the
/// cluster's files while the server runs is not a backup; `backup` makes those.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupExclusion {
    /// The cluster was there already, or this is not macOS.
    NotDue,
    Set,
    /// `tmutil` did not set it. The database works; the operator should know.
    Failed,
}

/// The line `anima-database serve` prints on standard error when [`Running::recovered`] is set.
/// The program that runs the hub's group reads it (one-click plan S4).
pub const RECOVERED_NOTICE: &str = "anima-database: recovered after an unclean stop";

/// A database that is up: its server, and the lock that keeps other copies away.
#[derive(Debug)]
pub struct Running {
    server: Server,
    /// Whether this start had to recover from a kill or a power cut.
    pub recovered: bool,
    pub backup_exclusion: BackupExclusion,
    // Declared after the server: the server stops before the lock is given up.
    _lock: Lock,
}

/// Brings the database of `layout` up with the PostgreSQL in `tree`.
///
/// A cluster of another major version is refused before anything is read from it or written
/// to it. This release has no upgrade step (decision D-6); the release that brings the next
/// PostgreSQL carries it.
///
/// # Errors
/// [`StartError::is_another_copy`] when another copy runs it,
/// [`StartError::is_version_refusal`] for a cluster of another version; otherwise the step
/// that failed.
pub fn start(
    tree: &Tree,
    layout: &Layout,
    settings: Settings,
    limits: Limits,
) -> Result<Running, StartError> {
    let lock = crate::lock(layout)?;
    let of_cluster = cluster::major_version(layout)?;
    if let Some(version) = of_cluster {
        match cluster::fit(version, tree.major_version()?) {
            Fit::Same => {}
            Fit::ClusterNewer { cluster, tree } => {
                return Err(StartError::ClusterNewer { cluster, tree });
            }
            Fit::ClusterOlder { cluster, tree } => {
                return Err(StartError::ClusterOlder { cluster, tree });
            }
        }
    }
    crate::stop_left_over(&lock, tree, layout, limits)?;
    let mut backup_exclusion = BackupExclusion::NotDue;
    if of_cluster.is_none() {
        cluster::create(&lock, tree, layout)?;
        backup_exclusion = exclude_from_time_machine(layout);
    }
    cluster::admit_this_user(&lock, layout)?;
    // A new cluster has no logins, also when URL files of an older cluster are still there.
    let has_logins = of_cluster.is_some()
        && urls::password(layout, Role::Owner)?.is_some()
        && urls::password(layout, Role::Hub)?.is_some();
    let recovered = !cluster::was_stopped_cleanly(tree, layout)?;
    let mut server = Server::start(&lock, tree, layout, settings)?;
    server.wait_until_ready(layout, limits)?;
    if !has_logins {
        roles::prepare(layout)?;
    }
    Ok(Running {
        server,
        recovered,
        backup_exclusion,
        _lock: lock,
    })
}

/// Marks the cluster folder as "not for Time Machine" (decision D-8). No administrator
/// rights: `tmutil addexclusion` without `-p` writes an attribute on the folder.
fn exclude_from_time_machine(layout: &Layout) -> BackupExclusion {
    if !matches!(Platform::HOST, Platform::MacOs) {
        return BackupExclusion::NotDue;
    }
    let set = std::process::Command::new("/usr/bin/tmutil")
        .env_clear()
        .arg("addexclusion")
        .arg(layout.cluster())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if set.is_ok_and(|status| status.success()) {
        BackupExclusion::Set
    } else {
        BackupExclusion::Failed
    }
}

/// What `status` tells: the two versions, and if a server accepts connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub tree: u32,
    /// `None` when no cluster is made yet.
    pub cluster: Option<u32>,
    pub server_accepts_connections: bool,
}

/// Looks at the database without a lock and without a change.
///
/// # Errors
/// When a version or `postmaster.pid` cannot be read.
pub fn status(tree: &Tree, layout: &Layout) -> Result<Status, StartError> {
    Ok(Status {
        tree: tree.major_version()?,
        cluster: cluster::major_version(layout)?,
        server_accepts_connections: crate::server::accepts_connections(layout)?,
    })
}

impl Running {
    /// The exit status when the server has ended by itself, `None` while it runs.
    ///
    /// # Errors
    /// When the operating system cannot say.
    pub fn ended(&mut self) -> Result<Option<ExitStatus>, ServerError> {
        self.server.ended()
    }

    /// Stops the server, then gives the lock up.
    ///
    /// # Errors
    /// When the server cannot be signalled or waited for.
    pub fn stop(self, limits: Limits) -> Result<Stopped, ServerError> {
        self.server.stop(limits)
    }
}
