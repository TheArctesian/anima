//! Runs the installed agent once (MET-019, D-10 d): an absolute program and fixed arguments, no
//! shell, an environment that holds only the variables the agent needs, the code on standard
//! input only, output thrown away, and a deadline after which the child is killed and reaped by
//! its own handle, as it is when the app stops it on exit. It logs nothing (LOG-007).

use std::{
    ffi::OsString,
    io::{ErrorKind, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

/// The variables the agent gets from the app, when they are set: its home (where `enrol` stores
/// the token and `install` writes the unit), and the user session bus `systemctl --user` needs.
const PASSED_ON: [&str; 2] = ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"];

const POLL: Duration = Duration::from_millis(20);

/// `ETXTBSY`, the same number on Linux and macOS.
const TEXT_BUSY: i32 = 26;
const BUSY_TRIES: u32 = 5;
const BUSY_PAUSE: Duration = Duration::from_millis(50);

/// One run of the agent.
pub struct Run<'a> {
    pub program: &'a Path,
    pub args: &'a [&'a str],
    /// Written to standard input, which is then closed; `None` gives it no input at all.
    pub input: Option<&'a [u8]>,
    pub env: &'a [(&'static str, OsString)],
    pub deadline: Duration,
    /// Set when the app exits: the child is then killed and reaped at once.
    pub stop: &'a AtomicBool,
}

/// Why a run did not succeed. Fixed text: no OS text, no argument and no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RunError {
    #[error("the agent could not start")]
    CouldNotStart,
    #[error("the agent could not be given its input")]
    Input,
    #[error("the agent exited with a failure")]
    Failed,
    #[error("the agent ran past its deadline")]
    TimedOut,
    #[error("the app stopped the agent as it exited")]
    Stopped,
    #[error("the agent's exit status could not be read")]
    Unreadable,
}

/// `enrol --hub <origin> -`: the code comes on standard input, never on the command line.
pub fn enrol_args(origin: &str) -> [&str; 4] {
    ["enrol", "--hub", origin, "-"]
}

/// `install`: the user unit or `LaunchAgent`; never `--system`, which needs root.
pub const INSTALL_ARGS: [&str; 1] = ["install"];

/// The agent's environment: `HOME` as given, then each of [`PASSED_ON`] that `lookup` finds and
/// is not empty. Nothing else of the app's environment reaches the agent.
pub fn agent_env(
    home: &Path,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> Vec<(&'static str, OsString)> {
    let mut env = vec![("HOME", home.as_os_str().to_owned())];
    for name in PASSED_ON {
        if let Some(value) = lookup(name).filter(|value| !value.is_empty()) {
            env.push((name, value));
        }
    }
    env
}

/// Runs `run` and waits for it until its deadline or its stop.
///
/// # Errors
/// [`RunError`]: the program could not start, take its input, or exit successfully in time, or
/// it was stopped. A child still running then is killed and reaped first.
pub fn run(run: &Run<'_>) -> Result<(), RunError> {
    if run.stop.load(Ordering::Acquire) {
        return Err(RunError::Stopped);
    }
    let mut child = start(run)?;
    if let Err(error) = hand_over(&mut child, run.input) {
        reap(child);
        return Err(error);
    }
    let end = Instant::now() + run.deadline;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => return Err(RunError::Failed),
            Ok(None) if run.stop.load(Ordering::Acquire) => {
                reap(child);
                return Err(RunError::Stopped);
            }
            Ok(None) if Instant::now() < end => thread::sleep(POLL),
            Ok(None) => {
                reap(child);
                return Err(RunError::TimedOut);
            }
            Err(_) => {
                reap(child);
                return Err(RunError::Unreadable);
            }
        }
    }
}

/// Starts the child. A binary that was just written can be "busy" for a moment while another
/// thread of this process forks with the file still open; that start is tried again.
fn start(run: &Run<'_>) -> Result<Child, RunError> {
    let mut tries = 0;
    loop {
        let started = Command::new(run.program)
            .args(run.args)
            .env_clear()
            .envs(run.env.iter().map(|(name, value)| (name, value)))
            .current_dir("/")
            .stdin(if run.input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match started {
            Err(error) if error.raw_os_error() == Some(TEXT_BUSY) && tries < BUSY_TRIES => {
                tries += 1;
                thread::sleep(BUSY_PAUSE);
            }
            started => return started.map_err(|_| RunError::CouldNotStart),
        }
    }
}

/// Writes `input` and closes standard input. A child that exits before reading leaves a broken
/// pipe; that is not an error here, as its exit status tells what happened.
fn hand_over(child: &mut Child, input: Option<&[u8]>) -> Result<(), RunError> {
    let Some(input) = input else {
        return Ok(());
    };
    let mut stdin = child.stdin.take().ok_or(RunError::Input)?;
    match stdin.write_all(input) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::BrokenPipe => Ok(()),
        Err(_) => Err(RunError::Input),
    }
}

/// Kills the child through its own handle and waits for it, so no other process is touched.
fn reap(mut child: Child) {
    // It may have exited already; either way the wait below reaps it.
    let _ = child.kill();
    let _ = child.wait();
}
