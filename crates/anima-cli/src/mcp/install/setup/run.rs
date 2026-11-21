//! Runs an agent's own program (MCP-035): fixed arguments, no shell, the home folder, a time limit.

use std::io::Read as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::Here;
use super::program::{Found, fixed_program};

/// The longest time an agent's own program may take; it is then stopped and counts as failed.
const RUN_LIMIT: Duration = Duration::from_secs(20);

/// What an agent's own program did: its exit and what it wrote, or why it did not run.
pub(super) enum Ran {
    Done {
        success: bool,
        stdout: Vec<u8>,
    },
    /// It ran past [`RUN_LIMIT`] and was stopped, or could not be started.
    Failed,
    /// A different user can replace it (MCP-033): it is not run.
    Replaceable,
    Missing,
}

/// Runs an agent's own program with fixed arguments and no shell (MCP-035), for at most
/// [`RUN_LIMIT`].
pub(super) fn run(here: &Here, name: &str, arguments: &[String]) -> Ran {
    let program = match fixed_program(here, name) {
        Found::Fixed(path) => path,
        Found::Replaceable => return Ran::Replaceable,
        Found::Missing => return Ran::Missing,
    };
    // In the home folder: an agent must not read the project configuration of the folder the
    // person happens to be in.
    let Ok(mut child) = Command::new(program)
        .args(arguments)
        .current_dir(&here.home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Ran::Failed;
    };
    let mut stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        if let Some(stdout) = stdout.as_mut() {
            let _ = stdout.take(1 << 20).read_to_end(&mut out);
        }
        out
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if start.elapsed() < RUN_LIMIT => {
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let stdout = reader.join().unwrap_or_default();
    status.map_or(Ran::Failed, |status| Ran::Done {
        success: status.success(),
        stdout,
    })
}
