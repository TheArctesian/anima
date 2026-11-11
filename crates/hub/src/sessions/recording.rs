//! Session recordings (RA-040, D-13, K4 §11): what a session's owner saw, kept encrypted on
//! the hub's disk with a key per recording.
//!
//! The file format is `format`, pure; SSH sessions are recorded as asciicast v2
//! (`asciicast`, D-12). `recorder` writes a session's recording, `store` keeps its row, and
//! `space` says whether the hub can record now. The catalogue's GraphQL part (`graphql`)
//! lists recordings, issues a playback ticket (`viewing`, held in `tickets`) and deletes one
//! (`deleting`, D-13). No log line or error text holds recording content, a key or a
//! recording's path (LOG-007).

mod asciicast;
mod deleting;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        unused_imports,
        reason = "the reader and opener serve playback (WP-48 T6)"
    )
)]
mod format;
mod graphql;
mod model;
mod recorder;
mod space;
mod store;
#[cfg(test)]
mod tests;
mod tickets;
mod viewing;

pub use self::graphql::{Mutation as RecordingsMutation, Query as RecordingsQuery};
pub(in crate::sessions) use self::tickets::Tickets;
#[expect(
    unused_imports,
    reason = "the tunnels (WP-48 T7, T8) and playback (T6) are the callers"
)]
pub(in crate::sessions) use self::{
    model::{RecordingState, StopReason, Stored},
    recorder::{Origin, Recorder, start},
    space::{CannotRecord, can_record},
    store::{get, states},
};

use crate::{credentials, ids::RecordingId, retention, sessions};

/// Why a recording could not start or end. The texts hold no path, key or content (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(in crate::sessions) enum Error {
    /// `ANIMA_RECORDING_ROOT` is empty.
    #[error("recording is not available on this hub")]
    Unavailable,
    #[error("no random bytes for a recording key")]
    Random(#[source] getrandom::Error),
    #[error("the recording key could not be sealed")]
    Vault(#[source] credentials::Error),
    #[error("the retention periods could not be read")]
    Periods(#[source] retention::Error),
    /// The start plus the recordings period is past the last time the hub can represent.
    #[error("the recording's expiry is out of range")]
    Expiry,
    /// The hub shut down before the recording's row was written.
    #[error("the hub is shutting down")]
    ShuttingDown,
    /// No recording has this id.
    #[error("recording {0} not found")]
    NotFound(RecordingId),
    /// Too many playback tickets are pending.
    #[error("no playback ticket can be issued now")]
    Busy,
    /// A stored row holds a value the hub does not know.
    #[error("a stored recording is not valid")]
    StoredValue,
    #[error("event store error")]
    Events(#[source] crate::events::Error),
    #[error("session store error")]
    Session(#[source] Box<sessions::Error>),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

impl Error {
    /// For the process log: what failed, as a fixed word, never the source text (LOG-007).
    pub(in crate::sessions) const fn fault(&self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Random(_) => "random",
            Self::Vault(_) => "vault",
            Self::Periods(_) => "periods",
            Self::Expiry => "expiry",
            Self::NotFound(_) => "not_found",
            Self::Busy => "busy",
            Self::StoredValue => "stored_value",
            Self::ShuttingDown => "shutting_down",
            Self::Events(_) => "events",
            Self::Session(_) => "session",
            Self::Database(_) => "database",
        }
    }
}
