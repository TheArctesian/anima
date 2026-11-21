//! Takes the daemon off this computer (HMD-056): the daemon runs `anima-daemon uninstall`, which
//! stops and deletes its unit and forgets its token and its key cache, then the shell deletes the
//! copy that it put in `~/.local/bin`. The daemon does the first part because the unit and the
//! token are its own (a Keychain item opens without a question only for the program that made
//! it); the shell does the second because it made the copy, as it does for the link program.

use std::{ffi::OsString, fs, io, path::Path, sync::atomic::AtomicBool, time::Duration};

use super::{
    DaemonError,
    run::{self, Run},
};

/// `uninstall`: the user unit or `LaunchAgent`; never a system unit, which needs root.
pub const UNINSTALL_ARGS: [&str; 1] = ["uninstall"];

/// What the removal needs; each path is given, so that a test runs it in a folder of its own.
pub struct Leave<'a> {
    /// The copy that the install made: `~/.local/bin/anima-daemon` of the user.
    pub copy: &'a Path,
    /// The daemon that the app carries; it runs `uninstall` when the copy is gone.
    pub carried: Option<&'a Path>,
    pub env: &'a [(&'static str, OsString)],
    pub deadline: Duration,
    pub stop: &'a AtomicBool,
}

/// Runs `uninstall` with the copy (or, when the copy is gone, the daemon that the app carries),
/// then deletes the copy. With neither, the app installed nothing that is still here, and
/// nothing is done. A second removal ends the same as the first. True when the copy was here:
/// then a daemon of this computer was installed, and the hub may list it.
///
/// # Errors
/// [`DaemonError::Uninstall`] when the daemon did not take its unit and its token out (the copy
/// then stays, so that the unit still finds its program and a later removal can run it);
/// [`DaemonError::TimedOut`] or [`DaemonError::Failed`] when it did not run to its end;
/// [`DaemonError::Remove`] when the copy could not be deleted.
pub fn remove_here(leave: &Leave<'_>) -> Result<bool, DaemonError> {
    let copied = leave.copy.is_file();
    let program = copied
        .then_some(leave.copy)
        .or(leave.carried.filter(|carried| carried.is_file()));
    let Some(program) = program else {
        return Ok(false);
    };
    let uninstall = Run {
        program,
        args: &UNINSTALL_ARGS,
        input: None,
        env: leave.env,
        deadline: leave.deadline,
        stop: leave.stop,
    };
    run::run(&uninstall).map_err(|error| DaemonError::of_run(error, DaemonError::Uninstall))?;
    match fs::remove_file(leave.copy) {
        Ok(()) => Ok(copied),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(copied),
        Err(_) => Err(DaemonError::Remove),
    }
}
