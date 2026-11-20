//! The one set-up step of "Keep awake with the lid closed" on macOS (AWK-003). It writes one
//! rule file that lets this user run exactly two commands as root with no password:
//! `pmset -a disablesleep 0` and `pmset -a disablesleep 1`. macOS asks for the administrator;
//! the shell never sees a password (INS-009). "Remove" deletes the file the same way.
//!
//! The operator starts each step. The script is fixed text: its only variable part is this
//! user's numeric ID.

use std::{ffi::OsStr, time::Duration};

use super::platform::{Os, Platform};
use crate::os_program::{self, AfterLimit, Limit, Stdout};

/// The `pmset` the rule names. Fixed: the rule must not follow a test's stand-in program.
const RULE_PMSET: &str = "/usr/bin/pmset";

/// What macOS shows in its password dialog.
const PROMPT: &str = "Anima needs an administrator to allow \"Keep awake with the lid closed\".";

/// The operator types a password in a system dialog.
const DIALOG: Limit = Limit {
    patience: Duration::from_secs(300),
    after: AfterLimit::Kill,
};

/// The `AppleScript`: the shell script and the prompt are arguments, so nothing is quoted into it.
const ASK: [&str; 3] = [
    "on run argv",
    "do shell script (item 1 of argv) with prompt (item 2 of argv) with administrator privileges",
    "end run",
];

mod model;
pub use model::{Lid, SetUpError};

/// The lid state of `platform`, from its OS, its desktop name and whether the rule exists.
#[must_use]
pub fn lid(platform: &Platform) -> Lid {
    match platform.os {
        Os::Mac if platform.rule.exists() => Lid::Ready,
        Os::Mac => Lid::NeedsSetUp,
        Os::Linux if decides_lid_itself(&platform.desktop) => Lid::Unsupported,
        Os::Linux => Lid::Ready,
    }
}

/// `XDG_CURRENT_DESKTOP` is a colon-separated list; KDE Plasma names itself `KDE`.
fn decides_lid_itself(desktop: &str) -> bool {
    desktop
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("KDE"))
}

/// The rule: user `#<id>` may run the two commands as root with no password.
#[must_use]
pub fn rule(user: u32) -> String {
    format!(
        "#{user} ALL=(root) NOPASSWD: {RULE_PMSET} -a disablesleep 0, {RULE_PMSET} -a disablesleep 1"
    )
}

/// The script root runs to write the rule: `visudo` checks the text before it is installed,
/// so a bad rule can never break `sudo`.
fn install_script(platform: &Platform) -> String {
    format!(
        "t=$(/usr/bin/mktemp /private/tmp/anima-stay-awake.XXXXXX) || exit 1; \
         /bin/echo '{rule}' > \"$t\" && /usr/sbin/visudo -cf \"$t\" > /dev/null && \
         /usr/bin/install -o root -g wheel -m 0440 \"$t\" '{file}'; \
         s=$?; /bin/rm -f \"$t\"; exit $s",
        rule = rule(platform.user),
        file = platform.rule.display(),
    )
}

/// The script root runs to delete the rule. `set_sleep_back`: Anima changed the sleep setting
/// and could not set it back; root does it here, because nothing can after the rule is gone.
fn remove_script(platform: &Platform, set_sleep_back: bool) -> String {
    let restore = if set_sleep_back {
        format!("{RULE_PMSET} -a disablesleep 0; ")
    } else {
        String::new()
    };
    format!("{restore}/bin/rm -f '{}'", platform.rule.display())
}

fn as_administrator(platform: &Platform, script: &str) -> Result<(), SetUpError> {
    if platform.os != Os::Mac {
        return Err(SetUpError::Unsupported);
    }
    let mut args: Vec<&OsStr> = Vec::new();
    for line in ASK {
        args.push("-e".as_ref());
        args.push(line.as_ref());
    }
    args.push(script.as_ref());
    args.push(PROMPT.as_ref());
    os_program::run(&platform.osascript, &args, DIALOG, Stdout::Discard)
        .map(drop)
        .map_err(|_| SetUpError::Refused)
}

/// Writes the rule file; macOS asks for an administrator.
///
/// # Errors
/// `Unsupported` off macOS; `Refused` when the dialog was cancelled or the script failed.
pub fn set_up(platform: &Platform) -> Result<(), SetUpError> {
    as_administrator(platform, &install_script(platform))
}

/// Deletes the rule file; macOS asks for an administrator.
///
/// # Errors
/// As `set_up`.
pub fn remove(platform: &Platform, set_sleep_back: bool) -> Result<(), SetUpError> {
    as_administrator(platform, &remove_script(platform, set_sleep_back))
}
