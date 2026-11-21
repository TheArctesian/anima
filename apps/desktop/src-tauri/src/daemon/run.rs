//! Runs the installed daemon once (MET-019, D-10 d): an absolute program and fixed arguments, no
//! shell, an environment that holds only the variables the daemon needs, the code on standard
//! input only, output thrown away, and a deadline after which the child and every process it
//! started are killed, and the child reaped, as they are when the app stops it on exit. It logs
//! nothing (LOG-007).

use std::{
    ffi::OsString,
    io::{ErrorKind, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

/// The variables the daemon gets from the app, when they are set: its home (where `enrol` stores
/// the token and `install` writes the unit), and the user session bus `systemctl --user` needs.
#[cfg(unix)]
const PASSED_ON: [&str; 2] = ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"];

#[cfg(windows)]
const PASSED_ON: [&str; 7] = [
    "SystemRoot",
    "WINDIR",
    "USERPROFILE",
    "LOCALAPPDATA",
    "APPDATA",
    "TEMP",
    "TMP",
];

const POLL: Duration = Duration::from_millis(20);

/// `ETXTBSY`, the same number on Linux and macOS.
const TEXT_BUSY: i32 = 26;
const BUSY_TRIES: u32 = 5;
const BUSY_PAUSE: Duration = Duration::from_millis(50);

/// One run of the daemon.
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
    #[error("the daemon could not start")]
    CouldNotStart,
    #[error("the daemon could not be given its input")]
    Input,
    #[error("the daemon exited with a failure")]
    Failed,
    #[error("the daemon ran past its deadline")]
    TimedOut,
    #[error("the app stopped the daemon as it exited")]
    Stopped,
    #[error("the daemon's exit status could not be read")]
    Unreadable,
}

/// `enrol --hub <origin> -`: the code comes on standard input, never on the command line.
pub fn enrol_args(origin: &str) -> [&str; 4] {
    ["enrol", "--hub", origin, "-"]
}

/// `install`: the user unit or `LaunchAgent`; never `--system`, which needs root.
pub const INSTALL_ARGS: [&str; 1] = ["install"];

/// The daemon's environment: `HOME` as given, then each of [`PASSED_ON`] that `lookup` finds and
/// is not empty. Nothing else of the app's environment reaches the daemon.
///
/// `XDG_CONFIG_HOME` is cleared on purpose (it is not in [`PASSED_ON`]): the daemon's home is then
/// `$HOME/.config/anima-daemon`, T6a's single place for it, which the unit's
/// `ReadWritePaths=%E/anima-daemon` names too; the daemon's `install` refuses a token stored
/// anywhere else.
pub fn daemon_env(
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

/// Runs `daemon_run` and waits for it until its deadline or its stop.
///
/// # Errors
/// [`RunError`]: the program could not start, take its input, or exit successfully in time, or
/// it was stopped. A child still running then is killed and reaped first.
pub fn run(daemon_run: &Run<'_>) -> Result<(), RunError> {
    finish(daemon_run, false).map(drop)
}

/// Runs `daemon_run` as [`run`] does and returns the first `limit` bytes of its standard output.
/// Only what the program wrote before it exited is read: the pipe is read without waiting, so a
/// process that it started and that keeps the pipe open cannot hold the app.
///
/// # Errors
/// [`RunError`] as [`run`]; [`RunError::Unreadable`] when the output cannot be read.
#[cfg(unix)]
pub fn output(daemon_run: &Run<'_>, limit: u64) -> Result<Vec<u8>, RunError> {
    use std::io::Read as _;
    let mut child = finish(daemon_run, true)?;
    let stdout = child.stdout.take().ok_or(RunError::Unreadable)?;
    let flags = nix::fcntl::fcntl(&stdout, nix::fcntl::FcntlArg::F_GETFL)
        .map_err(|_| RunError::Unreadable)?;
    let flags = nix::fcntl::OFlag::from_bits_truncate(flags) | nix::fcntl::OFlag::O_NONBLOCK;
    nix::fcntl::fcntl(&stdout, nix::fcntl::FcntlArg::F_SETFL(flags))
        .map_err(|_| RunError::Unreadable)?;
    let mut out = Vec::new();
    match stdout.take(limit).read_to_end(&mut out) {
        Ok(_) => Ok(out),
        // The bytes read before the pipe had no more stay in `out`.
        Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(out),
        Err(_) => Err(RunError::Unreadable),
    }
}

/// Starts `daemon_run`, with a pipe for its standard output when `capture`, and waits for it
/// until its deadline or its stop. Returns the child once it exited with success.
fn finish(daemon_run: &Run<'_>, capture: bool) -> Result<Child, RunError> {
    if daemon_run.stop.load(Ordering::Acquire) {
        return Err(RunError::Stopped);
    }
    let mut child = start(daemon_run, capture)?;
    #[cfg(windows)]
    let Ok(_job) = anima_windows::job::Job::start(child.id()) else {
        reap(child);
        return Err(RunError::CouldNotStart);
    };
    if let Err(error) = hand_over(&mut child, daemon_run.input) {
        reap(child);
        return Err(error);
    }
    let end = Instant::now() + daemon_run.deadline;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(child),
            Ok(Some(_)) => return Err(RunError::Failed),
            Ok(None) if daemon_run.stop.load(Ordering::Acquire) => {
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

/// Starts the child, with a pipe for its standard output when `capture`. A binary that was just
/// written can be "busy" for a moment while another thread of this process forks with the file
/// still open; that start is tried again.
fn start(daemon_run: &Run<'_>, capture: bool) -> Result<Child, RunError> {
    let mut tries = 0;
    loop {
        let mut command = Command::new(daemon_run.program);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            command.process_group(0);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as _;
            command.creation_flags(anima_windows::job::SUSPENDED);
        }
        let started = command
            .args(daemon_run.args)
            .env_clear()
            .envs(daemon_run.env.iter().map(|(name, value)| (name, value)))
            .current_dir("/")
            .stdin(if daemon_run.input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(if capture {
                Stdio::piped()
            } else {
                Stdio::null()
            })
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

/// Kills the child's process group, which holds the child and every process it started, then
/// reaps the child. The child is not reaped before the kill, so its pid still names its own group
/// and no other process can hold that number; nothing outside the group is touched.
fn reap(mut child: Child) {
    #[cfg(unix)]
    if let Ok(group) = i32::try_from(child.id()) {
        // A group that is gone is not an error: the wait below still reaps the child.
        let _ = nix::sys::signal::killpg(
            nix::unistd::Pid::from_raw(group),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
    let _ = child.kill();
    let _ = child.wait();
}
