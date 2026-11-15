//! What one copy does to bring the database up: take the lock, clear away what an earlier
//! copy left, make the cluster and the roles at the first run, start the server and prove it
//! ready.

use std::process::ExitStatus;

use crate::cluster::{self, ClusterError};
use crate::roles::{self, RolesError};
use crate::urls::{self, UrlError};
use crate::{Layout, Limits, Lock, Password, Role, Server, ServerError, Settings, Stopped, Tree};

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error(transparent)]
    Server(#[from] ServerError),
    #[error(transparent)]
    Cluster(#[from] ClusterError),
    #[error(transparent)]
    Roles(#[from] RolesError),
    #[error(transparent)]
    Url(#[from] UrlError),
    #[error("the operating system gave no random bytes")]
    Random(#[source] getrandom::Error),
    #[error(
        "the cluster is there and its superuser's URL file is not, so the hub's logins \
         cannot be made"
    )]
    SuperuserFileMissing,
}

/// A database that is up: its server, and the lock that keeps other copies away.
#[derive(Debug)]
pub struct Running {
    server: Server,
    /// Whether this start had to recover from a kill or a power cut.
    pub recovered: bool,
    // Declared after the server: the server stops before the lock is given up.
    _lock: Lock,
}

/// Brings the database of `layout` up with the PostgreSQL in `tree`.
///
/// # Errors
/// [`ServerError::AnotherCopy`] when another copy runs it; otherwise the step that failed.
pub fn start(
    tree: &Tree,
    layout: &Layout,
    settings: Settings,
    limits: Limits,
) -> Result<Running, StartError> {
    let lock = crate::lock(layout)?;
    crate::stop_left_over(&lock, tree, layout, limits)?;
    if cluster::major_version(layout)?.is_none() {
        let superuser = Password::random().map_err(StartError::Random)?;
        cluster::create(&lock, tree, layout, &superuser)?;
    }
    let recovered = !cluster::was_stopped_cleanly(tree, layout)?;
    let mut server = Server::start(&lock, tree, layout, settings)?;
    server.wait_until_ready(layout, limits)?;
    let has_logins = urls::password(layout, Role::Owner)?.is_some()
        && urls::password(layout, Role::Hub)?.is_some();
    if !has_logins {
        let superuser =
            urls::password(layout, Role::Superuser)?.ok_or(StartError::SuperuserFileMissing)?;
        roles::prepare(layout, &superuser)?;
    }
    Ok(Running {
        server,
        recovered,
        _lock: lock,
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
