//! `anima-local-hub run`: one group for one hub root, from the lock to the last state.

use std::fs::{File, TryLockError};
use std::io;

use anima_database::owner_only;

use crate::Folders;
use crate::child::Stop;
use crate::group::{Ending, Group, Plan, StartError};
use crate::log_file::{LOG_LIMIT, LogFile};
use crate::state::{self, State};

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// Another `run` holds this root.
    #[error("another copy runs the hub of this folder")]
    AnotherCopy,
    #[error("cannot {step} ({kind})")]
    Io {
        step: &'static str,
        kind: io::ErrorKind,
    },
}

fn io_error(step: &'static str) -> impl FnOnce(io::Error) -> RunError {
    move |error| RunError::Io {
        step,
        kind: error.kind(),
    }
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// Stopped on request. The unit does not start it again.
    Asked,
    /// A member ended or could not start. The unit starts the group again.
    Failed,
}

/// Held by the one `run` of a hub root. The operating system ends it with the process.
#[derive(Debug)]
pub struct Lock(#[expect(dead_code, reason = "the open file is the lock")] File);

/// Takes the lock of the hub root, and makes the root first when it is missing.
///
/// # Errors
/// [`RunError::AnotherCopy`] when a `run` holds it.
pub fn lock(folders: &Folders) -> Result<Lock, RunError> {
    owner_only::folder(folders.root()).map_err(io_error("make the hub's folder"))?;
    let file =
        owner_only::appended_file(&folders.lock_file()).map_err(io_error("open the lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(Lock(file)),
        Err(TryLockError::WouldBlock) => Err(RunError::AnotherCopy),
        Err(TryLockError::Error(source)) => Err(io_error("take the lock")(source)),
    }
}

/// Runs the group of `plan` until `stop` is asked for or the group fails. `ready` is called
/// once, when every member is up.
///
/// # Errors
/// [`RunError::AnotherCopy`], or when the hub's folders or the state cannot be written.
pub async fn run(plan: &Plan, mut stop: Stop, ready: impl FnOnce()) -> Result<Ended, RunError> {
    let folders = &plan.folders;
    let _lock = lock(folders)?;
    owner_only::folder(&folders.logs()).map_err(io_error("make the log folder"))?;
    let mut notes = LogFile::open(&folders.logs().join("local-hub.log"), LOG_LIMIT)
        .map_err(io_error("open the log"))?;
    let mut report = |state: State| {
        let _ = notes.line(&serde_json::to_string(&state).unwrap_or_default());
        state::write(&folders.state_file(), &state).map_err(io_error("write the state"))
    };
    report(State::Starting)?;
    let mut group = Group::default();
    let ending = match group.start(plan, &stop).await {
        Ok(up) => {
            report(State::Ready {
                hub: up.hub,
                recovered: up.recovered,
            })?;
            ready();
            group.until_ended(&mut stop).await
        }
        Err(StartError::StopAsked) => Ending::Asked,
        Err(StartError::Blocked(reason)) => {
            // Nothing of the group may run while the operator is told.
            std::mem::take(&mut group).stop().await;
            report(State::Blocked { reason })?;
            stop.asked().await;
            Ending::Asked
        }
        Err(error) => {
            group.stop().await;
            report(State::Failed {
                cause: error.to_string(),
            })?;
            return Ok(Ended::Failed);
        }
    };
    report(State::Stopping)?;
    group.stop().await;
    match ending {
        Ending::Asked => report(State::Stopped).map(|()| Ended::Asked),
        Ending::MemberEnded(name) => report(State::Failed {
            cause: format!("{name} ended"),
        })
        .map(|()| Ended::Failed),
    }
}

/// Whether a `run` holds the lock of this root now. It makes nothing.
#[must_use]
pub fn is_running(folders: &Folders) -> bool {
    let Ok(file) = File::open(folders.lock_file()) else {
        return false;
    };
    matches!(file.try_lock(), Err(TryLockError::WouldBlock))
}
