//! One fixed command `install` runs after the unit file is written, and how it is run (MET-019,
//! D-10): an absolute program, no shell, no input, a deadline, and a child that is killed by its
//! own handle when the deadline passes.

use std::io;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How long `systemctl` or `launchctl` may take before it is killed.
pub const STEP_DEADLINE: Duration = Duration::from_secs(30);

/// The only search path a command gets: the caller's own environment is not passed on.
const PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

/// How often a running command is looked at.
const POLL: Duration = Duration::from_millis(20);

/// One fixed command to run after the file is written.
#[derive(Debug, PartialEq, Eq)]
pub struct Step {
    pub program: &'static str,
    pub args: Vec<String>,
    /// A failure is expected and fine (`launchctl bootout` of an agent that is not loaded).
    pub may_fail: bool,
    /// Variables the command gets besides `PATH`: the session bus for `systemctl --user`.
    pub env: Vec<(&'static str, String)>,
    /// When the first run fails, wait this long and run it once more (`launchctl bootstrap`
    /// right after `bootout` can fail with EIO while launchd is still taking the old one out).
    pub retry_after: Option<Duration>,
}

impl Step {
    pub(super) fn new(program: &'static str, args: &[&str]) -> Self {
        Self {
            program,
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            may_fail: false,
            env: Vec::new(),
            retry_after: None,
        }
    }
}

/// Runs `step` with at most `deadline` per run, and once more after its pause when it has one
/// and the first run fails.
///
/// # Errors
/// The I/O error when the program cannot start or cannot be waited on; an error of kind
/// [`io::ErrorKind::TimedOut`] when it ran past `deadline` (it was then killed). `Ok(false)` is
/// a failure exit.
pub fn run_step(step: &Step, deadline: Duration) -> io::Result<bool> {
    let ok = run_once(step, deadline)?;
    match step.retry_after {
        Some(pause) if !ok => {
            thread::sleep(pause);
            run_once(step, deadline)
        }
        _ => Ok(ok),
    }
}

fn run_once(step: &Step, deadline: Duration) -> io::Result<bool> {
    let mut child = Command::new(step.program)
        .args(&step.args)
        .env_clear()
        .env("PATH", PATH)
        .envs(step.env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .spawn()?;
    let end = Instant::now() + deadline;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        let now = Instant::now();
        if now >= end {
            // Killed through its own handle, then reaped, so no other process is ever touched.
            let killed = child.kill();
            child.wait()?;
            killed?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("did not finish within {} s", deadline.as_secs_f32()),
            ));
        }
        thread::sleep(POLL.min(end - now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_019_a_step_that_runs_past_its_deadline_is_killed_and_is_an_error() {
        let step = Step::new("/bin/sleep", &["5"]);
        let started = Instant::now();
        let error = run_step(&step, Duration::from_millis(200)).expect_err("timed out");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "it was not waited out"
        );
    }

    #[test]
    fn met_019_a_step_gets_only_path_and_its_own_variables() {
        let dir = tempfile::tempdir().expect("a folder");
        let out = dir.path().join("env");
        let mut step = Step::new("/bin/sh", &["-c"]);
        step.args.push(format!("env > {}", out.display()));
        step.env
            .push(("XDG_RUNTIME_DIR", "/run/user/1000".to_owned()));
        assert!(run_step(&step, STEP_DEADLINE).expect("ran"));
        let seen = std::fs::read_to_string(&out).expect("written");
        assert!(seen.contains("XDG_RUNTIME_DIR=/run/user/1000\n"), "{seen}");
        assert!(!seen.contains("HOME="), "{seen}");
    }

    #[test]
    fn met_019_a_failed_step_with_a_retry_runs_once_more_after_its_pause() {
        let dir = tempfile::tempdir().expect("a folder");
        let count = dir.path().join("count");
        let mut step = Step::new("/bin/sh", &["-c"]);
        step.args
            .push(format!("echo run >> {}; exit 5", count.display()));
        assert!(!run_step(&step, STEP_DEADLINE).expect("ran"));
        step.retry_after = Some(Duration::from_millis(10));
        assert!(!run_step(&step, STEP_DEADLINE).expect("ran"));
        let runs = std::fs::read_to_string(&count).expect("written");
        assert_eq!(
            runs.lines().count(),
            3,
            "once without a retry, twice with one"
        );
    }
}
