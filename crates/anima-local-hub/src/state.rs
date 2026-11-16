//! What `run` last reported about the group, in a file that `status` reads. No secret.

use std::io::{self, Write as _};
use std::net::SocketAddr;
use std::path::Path;

use anima_database::owner_only;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// Why the group cannot start until the operator acts. `run` stays up and idle with it, so
/// that the unit does not start the group again in a loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Blocked {
    /// Another program runs this database (`anima-database` exit code 10).
    DatabaseInUse,
    /// The database is of another PostgreSQL major version (`anima-database` exit code 11).
    DatabaseVersion,
    /// The hub's key file is missing and the database is not new.
    KeyMissing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum State {
    Starting,
    Ready {
        /// Where the hub listens.
        hub: SocketAddr,
        /// The database replayed its log after an unclean stop; nothing saved was lost.
        recovered: bool,
    },
    Blocked {
        reason: Blocked,
    },
    Stopping,
    /// Stopped on request.
    Stopped,
    /// The group ended by itself; the unit starts it again.
    Failed {
        cause: String,
    },
}

/// What `anima-local-hub status` prints, as one JSON object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// The unit file and the copied programs are there.
    pub installed: bool,
    /// A `run` holds the lock of this root now.
    pub running: bool,
    /// The last report. When `running` is false it is what the last run left.
    #[serde(flatten)]
    pub last: Option<State>,
}

/// Replaces the state file with `state`, whole or not at all.
///
/// # Errors
/// When the file cannot be written.
pub fn write(file: &Path, state: &State) -> io::Result<()> {
    let mut being_written = file.as_os_str().to_owned();
    being_written.push(".new");
    let being_written = Path::new(&being_written);
    let mut new = owner_only::new_file(being_written)?;
    new.write_all(&serde_json::to_vec(state)?)?;
    std::fs::rename(being_written, file)
}

/// The last report; `None` when there is none or it cannot be read as one.
#[must_use]
pub fn read(file: &Path) -> Option<State> {
    serde_json::from_slice(&std::fs::read(file).ok()?).ok()
}
