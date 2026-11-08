//! Runs the processes of one build step (BLD-027, BLD-029, BLD-044): a scrubbed environment, its
//! own process group, its output passed on line by line, and cancellation that kills the whole
//! group. The lock is held only for short synchronous sections, never across an `await`.

use std::{
    ffi::OsString,
    path::Path,
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use tokio::io::{AsyncRead, AsyncReadExt};

use super::{error::BuilderError, lines::Lines, plan::Command};

/// Receives each output line of a step.
pub type Sink = Arc<dyn Fn(String) + Send + Sync>;

/// At most this many standard-output lines are kept for the caller to read (`git status`,
/// `lipo -archs`); every line still goes to the sink.
const KEPT_LINES_MAX: usize = 2000;
/// How long the output may take to close after the process group is gone.
const DRAIN: Duration = Duration::from_secs(5);
const CHUNK: usize = 8192;

#[derive(Debug, Default)]
struct Slot {
    busy: bool,
    group: Option<i32>,
    cancelled: bool,
}

/// The one build step that may run at a time on this computer (BLD-029), and its process group.
#[derive(Debug, Default)]
pub struct Running {
    slot: Mutex<Slot>,
}

/// Holds `Running` for one step, and where the step's lines go; dropping it frees the slot.
pub struct Claim<'a> {
    running: &'a Running,
    sink: Sink,
}

/// Sends SIGKILL to the process group `group`. A group that is gone is not an error.
fn kill_group(group: Option<i32>) {
    #[cfg(unix)]
    if let Some(group) = group {
        let _ = nix::sys::signal::killpg(
            nix::unistd::Pid::from_raw(group),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
    #[cfg(not(unix))]
    let _ = group;
}

impl Running {
    fn with_slot<T>(&self, change: impl FnOnce(&mut Slot) -> T) -> T {
        change(&mut self.slot.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Takes the slot for one step whose lines go to `sink`; `Busy` while another step holds it.
    pub fn claim(&self, sink: Sink) -> Result<Claim<'_>, BuilderError> {
        self.with_slot(|slot| {
            if slot.busy {
                return Err(BuilderError::Busy);
            }
            *slot = Slot {
                busy: true,
                ..Slot::default()
            };
            Ok(Claim {
                running: self,
                sink,
            })
        })
    }

    /// Kills the running step's process group and marks the step cancelled. Also called when
    /// the app exits, so no build outlives it.
    pub fn cancel(&self) {
        self.with_slot(|slot| {
            if slot.busy {
                slot.cancelled = true;
                kill_group(slot.group);
            }
        });
    }
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        self.running.with_slot(|slot| *slot = Slot::default());
    }
}

/// Reads one stream to its end, passing each line to `sink`; keeps up to `KEPT_LINES_MAX`.
async fn read_lines(stream: Option<impl AsyncRead + Unpin>, sink: Sink, keep: bool) -> Vec<String> {
    let mut kept = Vec::new();
    let Some(mut stream) = stream else {
        return kept;
    };
    let mut lines = Lines::default();
    let mut buffer = vec![0; CHUNK];
    let mut pass = |line: String| {
        if keep && kept.len() < KEPT_LINES_MAX {
            kept.push(line.clone());
        }
        sink(line);
    };
    while let Ok(read @ 1..) = stream.read(&mut buffer).await {
        lines.push(&buffer[..read]).into_iter().for_each(&mut pass);
    }
    lines.finish().into_iter().for_each(&mut pass);
    kept
}

impl Claim<'_> {
    /// Runs `command` as `program` with exactly `env`, and returns its kept standard-output
    /// lines. `Cancelled` once the step is cancelled; `StepFailed` for a non-zero exit.
    pub async fn run(
        &self,
        command: &Command,
        program: &Path,
        env: Vec<(OsString, OsString)>,
    ) -> Result<Vec<String>, BuilderError> {
        let mut process = tokio::process::Command::new(program);
        process
            .args(&command.args)
            .current_dir(&command.cwd)
            .env_clear()
            .envs(env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        process.process_group(0);
        let group = self.running.with_slot(|slot| {
            if slot.cancelled {
                return Err(BuilderError::Cancelled);
            }
            let mut child = process.spawn().map_err(|_| BuilderError::Failed)?;
            slot.group = child.id().and_then(|id| i32::try_from(id).ok());
            Ok((child.stdout.take(), child.stderr.take(), child))
        });
        let (stdout, stderr, mut child) = group?;
        let out = tokio::spawn(read_lines(stdout, Arc::clone(&self.sink), true));
        let err = tokio::spawn(read_lines(stderr, Arc::clone(&self.sink), false));
        let status = child.wait().await;
        // Anything the step left in its group would hold the output open.
        let cancelled = self.running.with_slot(|slot| {
            kill_group(slot.group.take());
            slot.cancelled
        });
        let kept = tokio::time::timeout(DRAIN, out).await;
        let _ = tokio::time::timeout(DRAIN, err).await;
        outcome(cancelled, status.ok(), kept.ok().and_then(Result::ok))
    }
}

fn outcome(
    cancelled: bool,
    status: Option<ExitStatus>,
    kept: Option<Vec<String>>,
) -> Result<Vec<String>, BuilderError> {
    if cancelled {
        return Err(BuilderError::Cancelled);
    }
    match status {
        Some(status) if status.success() => Ok(kept.unwrap_or_default()),
        _ => Err(BuilderError::StepFailed),
    }
}
