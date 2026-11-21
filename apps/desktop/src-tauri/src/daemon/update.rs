//! Updates the installed copy of the daemon with the app (HMD-055): when the daemon that the app
//! carries is newer than the copy in `~/.local/bin` ([`super::version`]), the copy is replaced
//! and the unit starts again, through the daemon's own `install`, as at the first install. The
//! token and the configuration stay: there is no new enrolment.
//!
//! The order: first the unit stops (`anima-daemon stop` of the carried daemon, because a daemon
//! of before has no such command), so that no daemon of before runs, and none starts a new holder
//! of the agent threads, once the new copy is in place. Then the copy is moved aside to
//! `anima-daemon.previous` (a rename: the same file, so a program that still has it open keeps
//! it, and macOS still finds its code signature valid). The new copy is written beside it under a
//! fresh name, flushed and renamed into place ([`super::placement::place`]): a new file, never a
//! write into the old one. Then `install` with the new copy writes the unit and starts it. When
//! the copy or the start fails, the copy of before goes back by a rename and its unit starts
//! again; the next start of the app tries again. No other process is signalled: the holder of
//! the agent threads (THR-113) and its programs stay, and the update does not wait for them. A
//! copy that is not installed is never installed here: that is the install of HMD-053.

use std::{
    ffi::OsString,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

use super::{
    DaemonError, placement,
    run::{self, INSTALL_ARGS, Run},
    trust::User,
    version::{self, Decision},
};

/// `stop`: the user unit stops; its file and the token stay (HMD-055).
pub const STOP_ARGS: [&str; 1] = ["stop"];

/// `--version` prints one line and touches nothing; a program that takes longer does not answer.
const VERSION_DEADLINE: Duration = Duration::from_secs(10);

/// What the update needs; each path is given, so that a test runs it in a folder of its own.
pub struct Swap<'a> {
    /// The installed copy: `~/.local/bin/anima-daemon` of the user.
    pub copy: &'a Path,
    /// The daemon that the app carries.
    pub carried: &'a Path,
    pub user: User,
    pub env: &'a [(&'static str, OsString)],
    /// The deadline of the daemon's `install`, which starts the unit again.
    pub restart: Duration,
    pub stop: &'a AtomicBool,
}

/// What the update did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Updated {
    /// No copy is installed, so nothing is updated.
    NoCopy,
    /// The version of the carried daemon could not be read, so it cannot be shown to be newer.
    CarriedUnknown,
    /// The copy has the version of the carried daemon.
    Unchanged,
    /// The copy is newer than the carried daemon, and stays.
    KeptNewer,
    /// The copy was replaced, and its unit runs the new one.
    Replaced,
}

/// Where the copy of before waits for the unit to start the new copy: `<copy>.previous`.
pub fn previous(copy: &Path) -> PathBuf {
    let mut name = copy.as_os_str().to_owned();
    name.push(".previous");
    PathBuf::from(name)
}

/// Compares the carried daemon with the installed copy and replaces the copy when the carried
/// one is newer (or the copy's version cannot be read, [`version::decide`]).
///
/// # Errors
/// [`DaemonError::Copy`], [`DaemonError::Untrusted`] or [`DaemonError::NotBundled`] when the new
/// copy was not put in place; [`DaemonError::Install`], [`DaemonError::TimedOut`] or
/// [`DaemonError::Failed`] when its unit did not start; [`DaemonError::Remove`] when the copy of
/// before could not be deleted after the start. In each case but the last, the copy of before
/// is back in place.
pub fn update_here(swap: &Swap<'_>) -> Result<Updated, DaemonError> {
    let aside = previous(swap.copy);
    if !swap.copy.is_file() && aside.is_file() {
        // An update stopped between the two renames: the copy of before goes back.
        put_back(swap.copy, &aside).map_err(|_| DaemonError::Copy)?;
    }
    if !swap.copy.is_file() {
        return Ok(Updated::NoCopy);
    }
    let read = |program| version::read(program, swap.env, VERSION_DEADLINE, swap.stop);
    let Some(carried) = read(swap.carried) else {
        return Ok(Updated::CarriedUnknown);
    };
    match version::decide(carried, read(swap.copy)) {
        Decision::Replace => replace(swap, &aside).map(|()| Updated::Replaced),
        Decision::Same => finish_earlier(swap, &aside).map(|()| Updated::Unchanged),
        Decision::KeepNewer => finish_earlier(swap, &aside).map(|()| Updated::KeptNewer),
    }
}

/// Stops the unit, moves the copy aside (unless an earlier copy already waits there), puts the
/// carried daemon in its place and starts the unit again. When the copy is not put in place, the
/// copy of before goes back and its unit starts again.
fn replace(swap: &Swap<'_>, aside: &Path) -> Result<(), DaemonError> {
    stop(swap)?;
    let failed = |error: DaemonError| {
        // The copy of before starts again; the error that matters is the first one.
        let _ = restart(swap, &real(swap.copy));
        Err(error)
    };
    if fs::symlink_metadata(aside).is_err() && fs::rename(swap.copy, aside).is_err() {
        return failed(DaemonError::Copy);
    }
    match placement::place(swap.carried, swap.copy, swap.user) {
        Ok(installed) => started(swap, &installed, aside),
        Err(error) => {
            if put_back(swap.copy, aside).is_err() {
                return Err(DaemonError::of_placement(error));
            }
            failed(DaemonError::of_placement(error))
        }
    }
}

/// `stop` with the carried daemon: the unit stops, and its file and the token stay. Nothing is
/// changed yet when it fails.
fn stop(swap: &Swap<'_>) -> Result<(), DaemonError> {
    let stop = Run {
        program: swap.carried,
        args: &STOP_ARGS,
        input: None,
        env: swap.env,
        deadline: swap.restart,
        stop: swap.stop,
    };
    run::run(&stop).map_err(|error| DaemonError::of_run(error, DaemonError::Failed))
}

/// The real path of `copy`, the path that the install runs; `copy` itself when it has none.
fn real(copy: &Path) -> PathBuf {
    fs::canonicalize(copy).unwrap_or_else(|_| copy.to_owned())
}

/// An update that a stop of the app cut after the new copy was put in place: its unit starts
/// again, so that it runs the copy that is there.
fn finish_earlier(swap: &Swap<'_>, aside: &Path) -> Result<(), DaemonError> {
    if aside.is_file() {
        return started(swap, &real(swap.copy), aside);
    }
    Ok(())
}

/// Starts the unit with `installed`, then deletes the copy of before. When the start fails, the
/// copy of before goes back and its unit starts again.
fn started(swap: &Swap<'_>, installed: &Path, aside: &Path) -> Result<(), DaemonError> {
    if let Err(error) = restart(swap, installed) {
        if put_back(swap.copy, aside).is_ok() {
            // The error that the person needs is the one of the new copy; the next start of
            // the app tries the update again.
            let _ = restart(swap, installed);
        }
        return Err(error);
    }
    match fs::remove_file(aside) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(DaemonError::Remove),
    }
}

/// `install` with `program`: it writes the unit again and starts it, so the unit runs the copy
/// that is in place now. The token stays where it is.
fn restart(swap: &Swap<'_>, program: &Path) -> Result<(), DaemonError> {
    let install = Run {
        program,
        args: &INSTALL_ARGS,
        input: None,
        env: swap.env,
        deadline: swap.restart,
        stop: swap.stop,
    };
    run::run(&install).map_err(|error| DaemonError::of_run(error, DaemonError::Install))
}

/// Renames the copy of before over the copy, and flushes the folder.
fn put_back(copy: &Path, aside: &Path) -> io::Result<()> {
    fs::rename(aside, copy)?;
    File::open(copy.parent().unwrap_or_else(|| Path::new("/")))?.sync_all()
}
