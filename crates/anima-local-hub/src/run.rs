//! `anima-local-hub run`: one group for one hub root, from the lock to the last state.

use std::fs::{File, TryLockError};
use std::io;
use std::time::Duration;

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

/// How long `lock` tries: `status` and `install` hold the lock for an instant to see if a
/// run has it, and a run that starts in that instant must not give up.
const LOCK_TRIES: u32 = 20;
const LOCK_PAUSE: Duration = Duration::from_millis(50);

/// Takes the lock of the hub root, and makes the root first when it is missing.
///
/// # Errors
/// [`RunError::AnotherCopy`] when a `run` holds it.
pub fn lock(folders: &Folders) -> Result<Lock, RunError> {
    owner_only::folder(folders.root()).map_err(io_error("make the hub's folder"))?;
    let file =
        owner_only::appended_file(&folders.lock_file()).map_err(io_error("open the lock"))?;
    for attempt in 1..=LOCK_TRIES {
        match file.try_lock() {
            Ok(()) => return Ok(Lock(file)),
            Err(TryLockError::WouldBlock) if attempt < LOCK_TRIES => {
                std::thread::sleep(LOCK_PAUSE);
            }
            Err(TryLockError::WouldBlock) => break,
            Err(TryLockError::Error(source)) => return Err(io_error("take the lock")(source)),
        }
    }
    Err(RunError::AnotherCopy)
}

/// What the group came to, before it is stopped.
enum Kept {
    Asked,
    Failed(String),
}

/// Runs the group of `plan` until `stop` is asked for or the group fails. `ready` is called
/// once, when every member is up. The group is stopped in order in every case, also when the
/// state cannot be written.
///
/// # Errors
/// [`RunError::AnotherCopy`], or when the hub's folders or the state cannot be written.
pub async fn run(plan: &Plan, mut stop: Stop, ready: impl FnOnce()) -> Result<Ended, RunError> {
    let folders = &plan.folders;
    let _lock = lock(folders)?;
    owner_only::folder(&folders.logs()).map_err(io_error("make the log folder"))?;
    let mut notes = LogFile::open(&folders.logs().join("local-hub.log"), LOG_LIMIT)
        .map_err(io_error("open the log"))?;
    let mut ready = Some(ready);
    let mut report = |state: State| {
        // The note is a help to read the log; the state file is what `status` tells.
        let _ = notes.line(&serde_json::to_string(&state).unwrap_or_default());
        state::write(&folders.state_file(), &state).map_err(io_error("write the state"))?;
        if matches!(state, State::Ready { .. })
            && let Some(ready) = ready.take()
        {
            ready();
        }
        Ok(())
    };
    let mut group = Group::default();
    let kept = keep(plan, &mut stop, &mut group, &mut report).await;
    let stopping = report(State::Stopping);
    let not_stopped = group.stop().await;
    let last = match &kept {
        Ok(Kept::Asked) | Err(_) => State::Stopped,
        Ok(Kept::Failed(cause)) => State::Failed {
            cause: cause.clone(),
        },
    };
    let stopped = report(last);
    for note in not_stopped {
        let _ = notes.line(&note);
    }
    let kept = kept?;
    stopping?;
    stopped?;
    Ok(match kept {
        Kept::Asked => Ended::Asked,
        Kept::Failed(_) => Ended::Failed,
    })
}

/// Starts the group and keeps it. The members it started are left in `group` for the caller
/// to stop.
async fn keep(
    plan: &Plan,
    stop: &mut Stop,
    group: &mut Group,
    mut report: impl FnMut(State) -> Result<(), RunError>,
) -> Result<Kept, RunError> {
    report(State::Starting)?;
    match group.start(plan, stop).await {
        Ok(up) => {
            report(State::Ready {
                hub: up.hub,
                recovered: up.recovered,
            })?;
            Ok(match group.until_ended(stop).await {
                Ending::Asked => Kept::Asked,
                Ending::MemberEnded(name) => Kept::Failed(format!("{name} ended")),
            })
        }
        Err(StartError::StopAsked) => Ok(Kept::Asked),
        Err(StartError::Blocked(reason)) => {
            // Nothing of the group may run while the operator is told.
            std::mem::take(group).stop().await;
            report(State::Blocked { reason })?;
            stop.asked().await;
            Ok(Kept::Asked)
        }
        Err(error) => Ok(Kept::Failed(error.to_string())),
    }
}

/// Whether a `run` holds the lock of this root now. It makes nothing.
#[must_use]
pub fn is_running(folders: &Folders) -> bool {
    let Ok(file) = File::open(folders.lock_file()) else {
        return false;
    };
    // A shared lock: two that look at the same time do not see each other as a run.
    matches!(file.try_lock_shared(), Err(TryLockError::WouldBlock))
}
