//! Runs one fixed read-only command (MET-002, CAP-022): fixed argv, no shell, no input, a time
//! limit, a cap on the output kept, and a failure that is an error value rather than a crash.
//! The command runs in a process group of its own, and the whole group is killed when it ends,
//! so nothing it started outlives it.

use std::io;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use rustix::process::{Pid, Signal, kill_process_group};
use tokio::io::{AsyncRead, AsyncReadExt as _};
use tokio::process::{Child, Command};
use tokio::time::timeout;

/// The most output kept from one command.
const OUTPUT_LIMIT: u64 = 1024 * 1024;

/// The only search path a command gets: the daemon's own environment is not passed on.
const PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

/// How long a command that closed its output may take to report its exit status.
const EXIT_GRACE: Duration = Duration::from_secs(1);

/// How long output still buffered is read after the command exits. A program it started in
/// the background may hold the output open; it is not waited for.
const DRAIN: Duration = Duration::from_millis(200);

/// A command the daemon runs: the program and its arguments are fixed text.
#[derive(Clone, Copy, Debug)]
pub struct FixedCommand {
    pub program: &'static str,
    pub args: &'static [&'static str],
    /// The longest the command may run; it is then killed.
    pub limit: Duration,
    /// `intel_gpu_top -J` never ends by itself: its output up to the limit is the answer.
    pub keep_cut_output: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{program} could not be started: {source}")]
    Start {
        program: &'static str,
        source: io::Error,
    },
    #[error("{program} did not finish in time")]
    Timeout { program: &'static str },
    #[error("{program} failed")]
    Failed { program: &'static str },
}

impl FixedCommand {
    /// Runs the command and returns what it printed (at most 1 MiB). It gets no input and its
    /// error output is thrown away; it gets a fixed `PATH` and `LC_ALL=C` (the number format its parser
    /// expects) and nothing else from the environment.
    ///
    /// # Errors
    /// [`CommandError`] when it cannot start, runs past its limit (unless
    /// [`FixedCommand::keep_cut_output`]) or exits with a failure.
    pub async fn run(&self) -> Result<Vec<u8>, CommandError> {
        let mut command = Command::new(self.program);
        command
            .args(self.args)
            .env_clear()
            .env("PATH", PATH)
            .env("LC_ALL", "C");
        let bounds = Bounds {
            limit: self.limit,
            keep_cut_output: self.keep_cut_output,
            output_limit: OUTPUT_LIMIT,
        };
        capture(&mut command, self.program, bounds).await
    }
}

/// How long a command may run and how much of what it prints is kept.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    /// The longest the command may run; it is then killed.
    pub limit: Duration,
    /// Whether the output up to the limit is the answer (see [`FixedCommand::keep_cut_output`]).
    pub keep_cut_output: bool,
    /// The most bytes of output kept.
    pub output_limit: u64,
}

/// Runs `command`, set up by the caller with its arguments and environment, with no input and
/// its error output thrown away, and returns what it printed within `bounds`. `program` names
/// it in an error. Reading stops when the command exits. The command and every process it
/// started in its group are killed when it ends, when the time runs out or when the caller is
/// dropped.
///
/// # Errors
/// [`CommandError`] when it cannot start, runs past its limit (unless
/// [`Bounds::keep_cut_output`]) or exits with a failure.
pub async fn capture(
    command: &mut Command,
    program: &'static str,
    bounds: Bounds,
) -> Result<Vec<u8>, CommandError> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|source| CommandError::Start { program, source })?;
    // Declared after `child`, so it is dropped first: the group dies before the child is dropped.
    let mut group = Group::of(&child);
    let Some(stdout) = child.stdout.take() else {
        return Err(CommandError::Failed { program });
    };
    let mut output = Vec::new();
    let mut reader = stdout.take(bounds.output_limit);
    // A read appends as it goes, so a cut leaves the output so far in `output`.
    let ended = timeout(
        bounds.limit,
        until_exit(&mut child, &mut reader, &mut output),
    )
    .await;
    let Ok(exited) = ended else {
        group.kill();
        // Reaped here; if that fails, `kill_on_drop` ends it with `child`.
        child.kill().await.ok();
        return if bounds.keep_cut_output {
            Ok(output)
        } else {
            Err(CommandError::Timeout { program })
        };
    };
    match exited {
        Ok(status) if status.success() => Ok(output),
        _ => Err(CommandError::Failed { program }),
    }
}

/// Reads `reader` into `output` until the command exits, then only what is already buffered.
async fn until_exit(
    child: &mut Child,
    reader: &mut (impl AsyncRead + Unpin),
    output: &mut Vec<u8>,
) -> io::Result<ExitStatus> {
    tokio::select! {
        read = reader.read_to_end(output) => {
            read?;
            timeout(EXIT_GRACE, child.wait()).await?
        }
        status = child.wait() => {
            let status = status?;
            if let Ok(read) = timeout(DRAIN, reader.read_to_end(output)).await {
                read?;
            }
            Ok(status)
        }
    }
}

/// The process group of a running command; dropping it kills every process in the group.
struct Group(Option<Pid>);

impl Group {
    fn of(child: &Child) -> Self {
        let leader = child
            .id()
            .and_then(|id| i32::try_from(id).ok())
            .and_then(Pid::from_raw);
        Self(leader)
    }

    /// Kills every process left in the group, once.
    fn kill(&mut self) {
        if let Some(group) = self.0.take() {
            // The group is often empty by now: the kill then fails with "no such process",
            // which is the outcome wanted, so the result is not needed.
            kill_process_group(group, Signal::KILL).ok();
        }
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn command(program: &'static str, args: &'static [&'static str]) -> FixedCommand {
        FixedCommand {
            program,
            args,
            limit: Duration::from_millis(300),
            keep_cut_output: false,
        }
    }

    #[tokio::test]
    async fn met_002_a_command_gives_its_output_with_no_shell() {
        // A shell would expand `$HOME`; the program gets the text.
        let out = command("echo", &["$HOME"]).run().await.expect("runs");
        assert_eq!(out, b"$HOME\n");
    }

    #[tokio::test]
    async fn met_002_a_missing_program_a_failure_and_a_timeout_are_errors_not_crashes() {
        assert!(matches!(
            command("anima-no-such-program", &[]).run().await,
            Err(CommandError::Start { .. })
        ));
        assert!(matches!(
            command("false", &[]).run().await,
            Err(CommandError::Failed { .. })
        ));
        assert!(matches!(
            command("sleep", &["5"]).run().await,
            Err(CommandError::Timeout { .. })
        ));
    }

    #[tokio::test]
    async fn met_016_a_command_that_never_ends_can_give_its_output_so_far() {
        let streaming = FixedCommand {
            keep_cut_output: true,
            ..command("tail", &["-f", "/etc/passwd"])
        };
        let out = streaming.run().await.expect("the cut output is the answer");
        assert!(out.len() > 1);
    }
}
