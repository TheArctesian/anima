use std::{ffi::OsStr, time::Duration};

use super::OperatingSystem;
use crate::os_program::{self, AfterLimit, Limit, Stdout};

/// Prints the macOS computer name. An absolute path, so `PATH` cannot swap it.
const SCUTIL: &str = "/usr/sbin/scutil";
/// How long `scutil` may take; it answers at once.
const PATIENCE: Duration = Duration::from_secs(2);
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
            whoami::hostname()
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
    let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    let limit = Limit {
        patience,
        after: AfterLimit::Kill,
    };
    let output = Stdout::Capture {
        max_bytes: MAX_OUTPUT,
    };
    os_program::run(program, &args, limit, output).ok()
}
