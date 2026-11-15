//! The holds that keep this computer awake (AWK-002, AWK-003). Each hold is a child program
//! that the OS ties to the app's process, so a hold ends when the app ends, also after a
//! failure (AWK-007).
//!
//! - Idle sleep, macOS: `caffeinate -i -w <pid>` holds a power assertion.
//! - Idle sleep, Linux: `systemd-inhibit --what=sleep` holds a `logind` inhibitor.
//! - Lid, Linux: `systemd-inhibit --what=handle-lid-switch`.
//! - Lid, macOS: `pmset -a disablesleep 1` as root through the set-up rule, with a watcher
//!   that sets it back when the app's process ends. A marker file records that Anima set it.
//!
//! Nothing here logs: the caller logs at its boundary.

use std::{
    ffi::OsStr,
    fs, io,
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};

use super::{
    platform::{Os, Platform},
    power,
};
use crate::os_program::{self, AfterLimit, Limit, Stdout};

/// A hold program that ends this soon after its start was refused by the OS.
const SETTLE: Duration = Duration::from_millis(250);

const POLL: Duration = Duration::from_millis(10);

/// `sudo -n` never waits for a password; `pmset` writes one setting.
const PMSET_WRITE: Limit = Limit {
    patience: Duration::from_secs(5),
    after: AfterLimit::Kill,
};

/// The watcher: waits for the followed process to end, then sets the sleep setting back.
/// `$0` is the process ID, `$1` is `sudo`, `$2` is `pmset`.
const WATCHER: &str = r#"while /bin/kill -0 "$0" 2>/dev/null; do /bin/sleep 2; done; exec "$1" -n "$2" -a disablesleep 0"#;

/// Why a hold was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HoldError {
    #[error("the hold program could not start")]
    CouldNotStart,
    #[error("the OS refused the hold")]
    Refused,
    #[error("the set-up step is not done")]
    NotSetUp,
    #[error("the sleep setting could not be changed")]
    Setting,
}

/// A running hold program, in a process group of its own. Dropping it ends the group.
#[derive(Debug)]
pub struct Hold(Child);

impl Hold {
    fn start(program: &str, args: &[&OsStr]) -> Result<Self, HoldError> {
        let child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|_| HoldError::CouldNotStart)?;
        let mut hold = Self(child);
        let started = Instant::now();
        while started.elapsed() < SETTLE {
            if !hold.alive() {
                return Err(HoldError::Refused);
            }
            thread::sleep(POLL);
        }
        Ok(hold)
    }

    /// Whether the hold program still runs.
    pub fn alive(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        if let Ok(group) = i32::try_from(self.0.id()) {
            // The group is already gone when the program ended by itself.
            let _ = killpg(Pid::from_raw(group), Signal::SIGTERM);
        }
        let _ = self.0.wait();
    }
}

/// The arguments of `systemd-inhibit` for one kind of inhibitor. Its command waits for the
/// followed process, so the inhibitor ends with the app.
fn inhibit_args(what: &str, followed: u32) -> Vec<String> {
    vec![
        format!("--what={what}"),
        "--who=Anima".into(),
        "--why=Stay awake".into(),
        "--mode=block".into(),
        "tail".into(),
        format!("--pid={followed}"),
        "-f".into(),
        "/dev/null".into(),
    ]
}

fn inhibit(platform: &Platform, what: &str) -> Result<Hold, HoldError> {
    let args = inhibit_args(what, platform.followed);
    let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    Hold::start(&platform.inhibit, &args)
}

/// Stops idle sleep of the computer; the display can sleep (AWK-002).
///
/// # Errors
/// `CouldNotStart` or `Refused` when the hold program did not stay running.
pub fn idle(platform: &Platform) -> Result<Hold, HoldError> {
    match platform.os {
        Os::Mac => {
            let followed = platform.followed.to_string();
            let args: [&OsStr; 3] = ["-i".as_ref(), "-w".as_ref(), followed.as_ref()];
            Hold::start(&platform.caffeinate, &args)
        }
        Os::Linux => inhibit(platform, "sleep"),
    }
}

/// What keeps the computer on with the lid closed (AWK-003).
#[derive(Debug)]
pub enum LidHold {
    /// A `logind` inhibitor (Linux).
    Inhibitor(Hold),
    /// Anima set `SleepDisabled` and sets it back (macOS).
    Setting { watcher: Hold },
    /// A different program had already set `SleepDisabled`: Anima leaves it alone (AWK-007).
    Borrowed,
}

/// Whether the report of `pmset -g` says that all sleep is disabled.
#[must_use]
pub fn sleep_disabled(report: &str) -> bool {
    report.lines().any(|line| {
        let mut words = line.split_whitespace();
        words.next() == Some("SleepDisabled") && words.next() == Some("1")
    })
}

fn set_sleep_disabled(platform: &Platform, disabled: bool) -> Result<(), HoldError> {
    let value = if disabled { "1" } else { "0" };
    let args: [&OsStr; 5] = [
        "-n".as_ref(),
        platform.pmset.as_ref(),
        "-a".as_ref(),
        "disablesleep".as_ref(),
        value.as_ref(),
    ];
    os_program::run(&platform.sudo, &args, PMSET_WRITE, Stdout::Discard)
        .map(drop)
        .map_err(|_| HoldError::Setting)
}

fn watcher(platform: &Platform) -> Result<Hold, HoldError> {
    let followed = platform.followed.to_string();
    let args: [&OsStr; 5] = [
        "-c".as_ref(),
        WATCHER.as_ref(),
        followed.as_ref(),
        platform.sudo.as_ref(),
        platform.pmset.as_ref(),
    ];
    Hold::start(&platform.shell, &args)
}

fn setting(platform: &Platform, marker: &Path) -> Result<LidHold, HoldError> {
    if !platform.rule.exists() {
        return Err(HoldError::NotSetUp);
    }
    let already =
        power::pmset_report(&platform.pmset, &["-g"]).is_some_and(|text| sleep_disabled(&text));
    if already && !marker.exists() {
        return Ok(LidHold::Borrowed);
    }
    // The marker is written first: a failure after the setting changed still leaves a record.
    fs::write(marker, b"").map_err(|_| HoldError::Setting)?;
    if let Err(error) = set_sleep_disabled(platform, true) {
        // The setting did not change, so there is nothing to set back.
        let _ = fs::remove_file(marker);
        return Err(error);
    }
    match watcher(platform) {
        Ok(watcher) => Ok(LidHold::Setting { watcher }),
        Err(error) => {
            let _ = restore(platform, marker);
            Err(error)
        }
    }
}

/// Keeps the computer on with the lid closed.
///
/// # Errors
/// `NotSetUp` on a Mac without the set-up rule; `Setting` when the sleep setting did not
/// change; `CouldNotStart` or `Refused` when a hold program did not stay running.
pub fn lid(platform: &Platform, marker: &Path) -> Result<LidHold, HoldError> {
    match platform.os {
        Os::Mac => setting(platform, marker),
        Os::Linux => inhibit(platform, "handle-lid-switch").map(LidHold::Inhibitor),
    }
}

/// Sets the sleep setting back when the marker says that Anima changed it, then removes the
/// marker. The marker stays when the setting did not change, so the next start tries again.
///
/// # Errors
/// `Setting` when the sleep setting did not change or the marker could not be removed.
pub fn restore(platform: &Platform, marker: &Path) -> Result<(), HoldError> {
    if !marker.exists() {
        return Ok(());
    }
    set_sleep_disabled(platform, false)?;
    match fs::remove_file(marker) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(HoldError::Setting),
        _ => Ok(()),
    }
}

/// Ends a lid hold: the computer sleeps with the lid closed as before (AWK-007).
///
/// # Errors
/// As `restore`.
pub fn release_lid(platform: &Platform, marker: &Path, hold: LidHold) -> Result<(), HoldError> {
    match hold {
        LidHold::Inhibitor(_) | LidHold::Borrowed => Ok(()),
        LidHold::Setting { watcher } => {
            // The watcher ends first, so it cannot write after a later hold.
            drop(watcher);
            restore(platform, marker)
        }
    }
}
