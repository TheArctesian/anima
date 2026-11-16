//! What `run` last reported about the group, in a file that `status` reads. No secret.

use std::io::{self, Write as _};
use std::net::SocketAddr;
use std::path::Path;
use std::time::SystemTime;

use anima_database::owner_only;
use chrono::{DateTime, Utc};
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
        /// The link to the devices (F-6). Absent in a state that a run before S6 wrote.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<LinkState>,
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

/// Where the link is: a link that is down is started again alone, and the hub still serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Link {
    /// Started, and not ready yet.
    Starting,
    Up,
    /// Ended, or never started; the next start waits for its pause.
    Down,
}

/// The link's state and when it came to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkState {
    pub state: Link,
    /// In whole seconds, written as RFC 3339.
    pub since: DateTime<Utc>,
}

impl LinkState {
    /// `state` since `time`, to the second. Pure: the caller reads the clock.
    #[must_use]
    pub fn at(state: Link, time: SystemTime) -> Self {
        let since = DateTime::<Utc>::from(time);
        let since = DateTime::from_timestamp(since.timestamp(), 0).unwrap_or(since);
        Self { state, since }
    }
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
