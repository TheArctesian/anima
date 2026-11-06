use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::OperatingSystem;

/// Prints the macOS computer name. An absolute path, so `PATH` cannot swap it.
const SCUTIL: &str = "/usr/sbin/scutil";
/// How long `scutil` may take; it answers at once.
const PATIENCE: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(10);
/// The most output read from the name program; a computer name is far shorter.
const MAX_OUTPUT: u64 = 1024;

/// The computer name: on macOS the one in Sharing settings, else (or when `scutil` gives
/// none) the host name without its domain. Empty when the OS gives neither.
pub(super) fn computer_name(os: OperatingSystem) -> String {
    let named = match os {
        OperatingSystem::Macos => output_of(SCUTIL, &["--get", "ComputerName"], PATIENCE)
            .and_then(|output| first_line(&output)),
        OperatingSystem::Linux => None,
    };
    named
        .or_else(|| {
            whoami::fallible::hostname()
                .ok()
                .and_then(|host| short_host_name(&host))
        })
        .unwrap_or_default()
}

/// The first line of `output`, trimmed; `None` when it is empty or not UTF-8.
pub(super) fn first_line(output: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(output).ok()?;
    let line = text.lines().next()?.trim();
    (!line.is_empty()).then(|| line.to_owned())
}

/// `host` up to its first dot, trimmed: `studio.local` is `studio`. `None` when empty.
pub(super) fn short_host_name(host: &str) -> Option<String> {
    let short = host.trim().split('.').next()?.trim();
    (!short.is_empty()).then(|| short.to_owned())
}

/// What `program` prints with `args` (no shell), if it exits successfully within `patience`.
/// A program still running then is killed. At most `MAX_OUTPUT` bytes are kept.
pub(super) fn output_of(program: &str, args: &[&str], patience: Duration) -> Option<Vec<u8>> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + patience;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
            Ok(None) | Err(_) => {
                // The output no longer matters; killing and waiting only reap the child.
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    let mut output = Vec::new();
    child
        .stdout
        .take()?
        .take(MAX_OUTPUT)
        .read_to_end(&mut output)
        .ok()?;
    Some(output)
}
