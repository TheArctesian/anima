//! Runs one fixed read-only command (MET-002, CAP-022): fixed argv, no shell, no input, a time
//! limit, a cap on the output kept, and a failure that is an error value rather than a crash.

use std::io;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt as _;
use tokio::process::Command;
use tokio::time::timeout;

/// The most output kept from one command.
const OUTPUT_LIMIT: u64 = 1024 * 1024;

/// The only search path a command gets: the agent's own environment is not passed on.
const PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

/// How long a finished command may take to report its exit status.
const EXIT_GRACE: Duration = Duration::from_secs(1);

/// A command the agent runs: the program and its arguments are fixed text.
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
/// it in an error. The child is killed when the time runs out or the caller is dropped.
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
        .kill_on_drop(true)
        .spawn()
        .map_err(|source| CommandError::Start { program, source })?;
    let Some(stdout) = child.stdout.take() else {
        return Err(CommandError::Failed { program });
    };
    let mut output = Vec::new();
    let mut reader = stdout.take(bounds.output_limit);
    // `read_to_end` appends as it reads, so a cut leaves the output so far in `output`.
    let finished = timeout(bounds.limit, reader.read_to_end(&mut output)).await;
    if finished.is_err() {
        // The child is killed here; if that fails, `kill_on_drop` ends it with `child`.
        child.kill().await.ok();
        return if bounds.keep_cut_output {
            Ok(output)
        } else {
            Err(CommandError::Timeout { program })
        };
    }
    if finished.is_ok_and(|read| read.is_err()) {
        return Err(CommandError::Failed { program });
    }
    match timeout(EXIT_GRACE, child.wait()).await {
        Ok(Ok(status)) if status.success() => Ok(output),
        _ => Err(CommandError::Failed { program }),
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
