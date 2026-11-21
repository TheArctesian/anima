//! The version of a daemon program, read from `anima-daemon --version`, and the rule that decides
//! whether the daemon that the app carries replaces the installed copy (HMD-055). The rule is
//! pure; only [`read`] runs a program. File times are never compared.

use std::{ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Duration};

use super::run::{self, Run};

/// `--version`: the daemon prints `anima-daemon MAJOR.MINOR.PATCH` and touches nothing.
pub const VERSION_ARGS: [&str; 1] = ["--version"];

/// The most bytes read of the answer: the line is under 40.
const LONGEST: u64 = 128;

/// What `--version` prints before the number.
const NAME: &str = "anima-daemon ";

/// A release version: major, minor and patch, compared as numbers (R-201 makes the patch the
/// count of commits on `main`).
pub type Version = [u64; 3];

/// What the update does with the installed copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The carried daemon is newer, or the installed version cannot be read.
    Replace,
    /// Both have the same version.
    Same,
    /// The installed copy is newer: an older daemon never goes over it.
    KeepNewer,
}

/// The version in the answer of `--version`: the name, one space, three numbers, and nothing
/// after them but the end of the line.
pub fn parse(answer: &[u8]) -> Option<Version> {
    let text = std::str::from_utf8(answer).ok()?;
    let text = text.strip_suffix('\n').unwrap_or(text);
    let mut parts = text.strip_prefix(NAME)?.split('.').map(|part| {
        part.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| part.parse::<u64>().ok())
            .flatten()
    });
    let found = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(found)
}

/// The rule of HMD-055. An installed copy whose version cannot be read counts as older: each
/// daemon from the first one that prints its version answers `--version`, so a copy that does
/// not answer is a daemon from before that (all older than the first app that updates), or a
/// copy that does not run. Replacing either is safe; keeping it would leave such a computer
/// with no update for ever.
pub fn decide(carried: Version, installed: Option<Version>) -> Decision {
    match installed {
        None => Decision::Replace,
        Some(installed) if carried > installed => Decision::Replace,
        Some(installed) if carried == installed => Decision::Same,
        Some(_) => Decision::KeepNewer,
    }
}

/// The version of `program`, by its `--version`; None when it does not run, fails, runs past
/// `deadline` or prints something else.
pub fn read(
    program: &Path,
    env: &[(&'static str, OsString)],
    deadline: Duration,
    stop: &AtomicBool,
) -> Option<Version> {
    let asked = Run {
        program,
        args: &VERSION_ARGS,
        input: None,
        env,
        deadline,
        stop,
    };
    run::output(&asked, LONGEST)
        .ok()
        .and_then(|answer| parse(&answer))
}
