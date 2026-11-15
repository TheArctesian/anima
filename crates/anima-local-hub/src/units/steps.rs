//! One command of the operating system's unit tool, run with a fixed environment and a limit.

use std::io;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How long a command may take. A stop waits for the group to stop in order.
pub const STEP_LIMIT: Duration = Duration::from_secs(120);

const LAUNCHCTL: &str = "/bin/launchctl";
const SYSTEMCTL: &str = "/usr/bin/systemctl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub program: &'static str,
    pub args: Vec<String>,
    /// A failure is not an error: the unit may not be loaded.
    pub may_fail: bool,
    /// Run it once more after this pause when it fails. `launchctl bootstrap` right after
    /// `bootout` can fail with an input/output error.
    pub retry_after: Option<Duration>,
}

impl Step {
    fn new(program: &'static str, args: &[&str]) -> Self {
        Self {
            program,
            args: args.iter().map(|&arg| arg.to_owned()).collect(),
            may_fail: false,
            retry_after: None,
        }
    }

    pub(super) fn launchctl(args: &[&str]) -> Self {
        Self::new(LAUNCHCTL, args)
    }

    pub(super) fn systemctl(args: &[&str]) -> Self {
        let mut all = vec!["--user"];
        all.extend_from_slice(args);
        Self::new(SYSTEMCTL, &all)
    }

    pub(super) const fn may_fail(mut self) -> Self {
        self.may_fail = true;
        self
    }

    pub(super) const fn retried_after(mut self, pause: Duration) -> Self {
        self.retry_after = Some(pause);
        self
    }
}

/// Runs `step` with only `env` and a fixed `PATH`; `false` when it exits with a failure.
///
/// # Errors
/// When the program cannot be run, or does not end in `limit` (it is killed).
pub fn run_step(step: &Step, env: &[(&'static str, String)], limit: Duration) -> io::Result<bool> {
    if run_once(step, env, limit)? {
        return Ok(true);
    }
    let Some(pause) = step.retry_after else {
        return Ok(false);
    };
    thread::sleep(pause);
    run_once(step, env, limit)
}

fn run_once(step: &Step, env: &[(&'static str, String)], limit: Duration) -> io::Result<bool> {
    let mut child = Command::new(step.program)
        .args(&step.args)
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
        .envs(env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        if started.elapsed() > limit {
            child.kill()?;
            child.wait()?;
            return Err(io::ErrorKind::TimedOut.into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}
