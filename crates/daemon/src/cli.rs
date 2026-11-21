//! The command line: `anima-daemon enrol <code|-> [--hub <origin>]`, `anima-daemon run`,
//! `anima-daemon install [--system]` (MET-012, MET-019), `anima-daemon uninstall` (HMD-056),
//! `anima-daemon stop` (HMD-055),
//! `anima-daemon key read <name>` and `anima-daemon key forget` (K-E5 Section 2.1), and
//! `anima-daemon --version`, which the desktop app reads to update the copy it installed (HMD-055).

use std::io::{BufRead, Read as _};
use std::path::PathBuf;

use daemon_core::wire::{EnrolmentCode, is_key_name};
use zeroize::Zeroizing;

use crate::config::origin::{Origin, OriginError};
use crate::units::plan::Target;

pub const USAGE: &str = "usage: anima-daemon enrol <code | -> [--hub <origin>]
       anima-daemon run [--hosted-hub <data folder>]
       anima-daemon install [--system]
       anima-daemon uninstall
       anima-daemon stop
       anima-daemon key read <name>
       anima-daemon key forget
       anima-daemon threads
       anima-daemon holder <folder> [<bytes kept for each thread>]
       anima-daemon --version";

/// What `--version` prints: the name and the release version (R-122), one line. The desktop app
/// reads it to decide whether the daemon it carries is newer than the installed copy (HMD-055),
/// so its form does not change.
#[must_use]
pub fn version_line() -> String {
    format!("anima-daemon {}", env!("CARGO_PKG_VERSION"))
}

/// Printed when the code is on the command line, where `ps` and the shell history show it.
pub const ARGV_WARNING: &str = "warning: a code on the command line shows in process lists and shell history; next time run `anima-daemon enrol -` and paste the code at the hidden prompt";

/// What the operator asked for.
pub enum Command {
    /// Exchange an enrolment code for a token and store both (K8 Section 3).
    Enrol { code: CodeSource, hub: Origin },
    /// Sample and push until stopped (K8 Sections 4 to 7). With `--hosted-hub DIR`, the daemon
    /// is the one in the container of a hosted hub, whose data is in `DIR` (HMD-071).
    Run { hosted_hub: Option<PathBuf> },
    /// Write the unit that starts `run` at login or boot, and start it (MET-019, D-10).
    Install { target: Target },
    /// Stop and delete the user unit, then forget the token and the key cache (HMD-056).
    Uninstall,
    /// Stop the user unit and keep its file and the token, before the copy is replaced (HMD-055).
    Stop,
    /// Write the cached value of one operational key to standard output (K-E5 Section 2.1).
    KeyRead { name: String },
    /// Delete the whole key cache, as when the daemon leaves the machine.
    KeyForget,
    /// Drive the agent threads of this computer, with no hub and no enrolment (stage 1b of
    /// the agent threads; `run` drives them too).
    Threads,
    /// Hold the agent programs of the threads; the daemon starts it (THR-113).
    Holder {
        folder: std::path::PathBuf,
        limit: usize,
    },
    /// Print [`version_line`]: no home, no token and no network.
    Version,
}

/// Where the enrolment code comes from: the command line, or standard input for `-`.
pub enum CodeSource {
    Given(EnrolmentCode),
    Stdin,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CliError {
    #[error("{USAGE}")]
    Usage,
    #[error("that is not an enrolment code (it starts with anima_enrol_)")]
    NotACode,
    #[error("--hub: {0}")]
    Hub(#[from] OriginError),
    #[error("that is not a key name (a-z, 0-9, _, . and -, starting with a letter)")]
    NotAKeyName,
}

/// Reads the arguments after the program name.
///
/// # Errors
/// [`CliError`] for a command, option or code that does not read. No error holds the code.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, CliError> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("run") => run(args),
        Some("uninstall") if args.next().is_none() => Ok(Command::Uninstall),
        Some("stop") if args.next().is_none() => Ok(Command::Stop),
        Some("enrol") => enrol(args),
        Some("install") => install(args),
        Some("key") => key(args),
        Some("holder") => holder(args),
        Some("threads") if args.next().is_none() => Ok(Command::Threads),
        Some("--version") if args.next().is_none() => Ok(Command::Version),
        _ => Err(CliError::Usage),
    }
}

/// `holder <folder> [<bytes>]`: an absolute folder, and the limit of each thread (0: the default).
fn holder(mut args: impl Iterator<Item = String>) -> Result<Command, CliError> {
    let folder = args
        .next()
        .filter(|folder| std::path::Path::new(folder).is_absolute());
    let limit = match args.next() {
        None => Some(0),
        Some(bytes) => bytes.parse().ok(),
    };
    match (folder, limit, args.next()) {
        (Some(folder), Some(limit), None) => Ok(Command::Holder {
            folder: folder.into(),
            limit,
        }),
        _ => Err(CliError::Usage),
    }
}

fn run(mut args: impl Iterator<Item = String>) -> Result<Command, CliError> {
    let hosted_hub = match (args.next().as_deref(), args.next()) {
        (None, _) => None,
        (Some("--hosted-hub"), Some(folder)) if folder.starts_with('/') => {
            Some(PathBuf::from(folder))
        }
        _ => return Err(CliError::Usage),
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    Ok(Command::Run { hosted_hub })
}

fn key(mut args: impl Iterator<Item = String>) -> Result<Command, CliError> {
    let command = match (args.next().as_deref(), args.next()) {
        (Some("read"), Some(name)) if is_key_name(&name) => Command::KeyRead { name },
        (Some("read"), Some(_)) => return Err(CliError::NotAKeyName),
        (Some("forget"), None) => return Ok(Command::KeyForget),
        _ => return Err(CliError::Usage),
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    Ok(command)
}

fn install(mut args: impl Iterator<Item = String>) -> Result<Command, CliError> {
    let target = match args.next().as_deref() {
        None => Target::User,
        Some("--system") => Target::System,
        Some(_) => return Err(CliError::Usage),
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    Ok(Command::Install { target })
}

fn enrol(args: impl Iterator<Item = String>) -> Result<Command, CliError> {
    let mut code = None;
    let mut hub = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--hub" {
            hub = Some(args.next().ok_or(CliError::Usage)?);
        } else if let Some(text) = arg.strip_prefix("--hub=") {
            hub = Some(text.to_owned());
        } else if code.is_none() && (arg == "-" || !arg.starts_with("--")) {
            code = Some(arg);
        } else {
            return Err(CliError::Usage);
        }
    }
    let code = match code.ok_or(CliError::Usage)?.as_str() {
        "-" => CodeSource::Stdin,
        text => CodeSource::Given(EnrolmentCode::parse(text).ok_or(CliError::NotACode)?),
    };
    let hub = match hub {
        Some(text) => Origin::parse(&text)?,
        None => Origin::default_hub(),
    };
    Ok(Command::Enrol { code, hub })
}

/// The most bytes read for a code from standard input: a code is under 40.
const CODE_INPUT_LIMIT: usize = 256;

/// The enrolment code on the first line of `input`: at most 256 bytes are read, and the line
/// may end in LF or CRLF.
///
/// # Errors
/// [`CliError::NotACode`] for an empty line, a line that is not a code (a code split by a
/// newline leaves only its first part), or 256 bytes with no newline. The error holds none of
/// the input.
///
/// The code also stays in the `StdinLock`'s own buffer, which is not wiped (LOG-007); the
/// process is short-lived, and only reads the one line.
pub fn read_code(mut input: impl BufRead) -> Result<EnrolmentCode, CliError> {
    // The buffer has room for every byte that can come, so it never moves and is wiped whole.
    let mut line = Zeroizing::new(String::with_capacity(CODE_INPUT_LIMIT + 1));
    input
        .by_ref()
        .take(CODE_INPUT_LIMIT as u64)
        .read_line(&mut line)
        .map_err(|_| CliError::NotACode)?;
    code_from_line(&line)
}

/// The code in `line`, which may end in one LF or CRLF (a terminal read gives none).
///
/// # Errors
/// [`CliError::NotACode`] when the rest is not a code.
pub fn code_from_line(line: &str) -> Result<EnrolmentCode, CliError> {
    let text = line.strip_suffix('\n').unwrap_or(line);
    let text = text.strip_suffix('\r').unwrap_or(text);
    EnrolmentCode::parse(text).ok_or(CliError::NotACode)
}

#[cfg(test)]
mod tests;
