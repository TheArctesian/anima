//! `anima-daemon uninstall` (HMD-056): stops the user unit or `LaunchAgent` of the daemon,
//! deletes its file, then has the caller forget the token and the key cache. Each part that is
//! not there is not an error, so a second run ends the same as the first. The file is the one
//! `install` writes ([`super::plan`]); the commands are run by the caller's `run`.
//!
//! `anima-daemon stop` (HMD-055) only stops the unit: the desktop app runs it before it replaces
//! the copy of the daemon, so that no daemon of before runs once the new copy is in place.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::plan::{self, PlanError, Target};
use super::render::Unit;
use super::session::{self, NoSessionBus};
use super::step::Step;
use crate::keys::store::KeyStoreError;
use crate::readers::Platform;
use crate::token_store::TokenStoreError;

#[derive(Debug, thiserror::Error)]
pub enum UninstallError {
    #[error("{0}")]
    Plan(#[from] PlanError),
    #[error("{0}")]
    NoSessionBus(#[from] NoSessionBus),
    #[error("deleting {}: {source}", path.display())]
    Delete { path: PathBuf, source: io::Error },
    #[error("{program} {} did not succeed", args.join(" "))]
    Step {
        program: &'static str,
        args: Vec<String>,
    },
    #[error("{program} did not complete: {source}")]
    Run {
        program: &'static str,
        source: io::Error,
    },
    #[error("the token store: {0}")]
    Token(#[from] TokenStoreError),
    #[error("the key cache: {0}")]
    Keys(#[from] KeyStoreError),
}

/// The user whose unit is taken out, on this platform.
#[derive(Clone, Copy, Debug)]
pub struct Leaving {
    pub platform: Platform,
    pub uid: u32,
}

/// What `uninstall` did: the unit file it looked at, and whether that file was there.
#[derive(Debug, PartialEq, Eq)]
pub struct Uninstalled {
    pub file: PathBuf,
    pub was_installed: bool,
}

/// The commands around the deletion of the unit file: `stop` before, `reload` after.
#[derive(Debug, PartialEq, Eq)]
struct Removal {
    file: PathBuf,
    stop: Vec<Step>,
    reload: Vec<Step>,
}

/// The removal for the user unit of `user`; `env` looks up environment variables. A unit that
/// is not loaded makes the stop fail, which is fine.
fn removal(user: Leaving, env: impl Fn(&str) -> Option<String>) -> Result<Removal, PlanError> {
    let install = plan::plan(Target::User, user.platform, user.uid, env)?;
    let (stop, reload) = if install.unit == Unit::LaunchAgent {
        (vec![bootout(user.uid)], Vec::new())
    } else {
        let disable = Step::new(
            "/usr/bin/systemctl",
            &["--user", "disable", "--now", "anima-daemon.service"],
        );
        let reload = Step::new("/usr/bin/systemctl", &["--user", "daemon-reload"]);
        (vec![disable], vec![reload])
    };
    let stop = stop
        .into_iter()
        .map(|mut step| {
            step.may_fail = true;
            step
        })
        .collect();
    Ok(Removal {
        file: install.file,
        stop,
        reload,
    })
}

/// `launchctl bootout` of the `LaunchAgent` of the daemon of user `uid`.
fn bootout(uid: u32) -> Step {
    let mut bootout = Step::new("/bin/launchctl", &["bootout"]);
    bootout.args.push(format!("gui/{uid}/ing.anima.daemon"));
    bootout
}

/// Stops the user unit of the user of this process, with the commands of this computer
/// (`anima-daemon stop`, HMD-055). The unit file, the token and the key cache stay.
///
/// # Errors
/// [`UninstallError::Run`] when the user of this process cannot be read; else as [`stop`].
#[cfg(unix)]
pub fn stop_here() -> Result<(), UninstallError> {
    let uid =
        crate::config::private_file::running_user().map_err(|source| UninstallError::Run {
            program: "reading the user of this process",
            source,
        })?;
    let user = Leaving {
        platform: Platform::current(),
        uid,
    };
    stop(
        user,
        |name| std::env::var(name).ok(),
        |step| super::step::run_step(step, super::step::STEP_DEADLINE),
    )
}

/// Stops the user unit of the daemon and leaves its file, the token and the key cache: the next
/// `install` starts it again. Only the unit is stopped (`launchctl bootout`, or
/// `systemctl --user stop`); no other process is signalled. A unit that is not loaded makes the
/// stop fail, which is fine; on Linux, with no session bus there is no user manager to stop.
///
/// # Errors
/// [`UninstallError`] when there is no home folder, or the command could not run to its end.
pub fn stop(
    user: Leaving,
    env: impl Fn(&str) -> Option<String>,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<(), UninstallError> {
    let install = plan::plan(Target::User, user.platform, user.uid, &env)?;
    let mut step = if install.unit == Unit::LaunchAgent {
        bootout(user.uid)
    } else {
        let Ok(bus) = session::session_bus(&env, user.uid) else {
            return Ok(());
        };
        let mut stop = Step::new(
            "/usr/bin/systemctl",
            &["--user", "stop", "anima-daemon.service"],
        );
        stop.env = bus;
        stop
    };
    step.may_fail = true;
    run_all(&[step], &mut run)
}

/// Stops and deletes the user unit of the daemon, then calls `forget` (the token and the key
/// cache). Each command goes through `run` (`Ok(false)` is a failure exit). On Linux the
/// commands get the session bus; with no bus and no unit file there is no unit to stop.
///
/// # Errors
/// [`UninstallError`] for the first part that does not succeed; the token is forgotten only
/// once the unit is out, so a daemon that still runs never loses its token from under it.
pub fn uninstall(
    user: Leaving,
    env: impl Fn(&str) -> Option<String>,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
    forget: impl FnOnce() -> Result<(), UninstallError>,
) -> Result<Uninstalled, UninstallError> {
    let mut removal = removal(user, &env)?;
    let was_installed = fs::symlink_metadata(&removal.file).is_ok();
    let steps_run = if user.platform == Platform::Linux {
        match session::session_bus(&env, user.uid) {
            Ok(bus) => {
                for step in removal.stop.iter_mut().chain(&mut removal.reload) {
                    step.env.clone_from(&bus);
                }
                true
            }
            Err(error) if was_installed => return Err(error.into()),
            Err(_) => false,
        }
    } else {
        true
    };
    if steps_run {
        run_all(&removal.stop, &mut run)?;
    }
    delete(&removal.file)?;
    if steps_run {
        run_all(&removal.reload, &mut run)?;
    }
    forget()?;
    Ok(Uninstalled {
        file: removal.file,
        was_installed,
    })
}

/// Deletes `file` (a link is deleted, not followed); a file that is not there is not an error.
fn delete(file: &Path) -> Result<(), UninstallError> {
    match fs::remove_file(file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(UninstallError::Delete {
            path: file.to_owned(),
            source,
        }),
    }
}

fn run_all(
    steps: &[Step],
    run: &mut impl FnMut(&Step) -> io::Result<bool>,
) -> Result<(), UninstallError> {
    for step in steps {
        let ok = run(step).map_err(|source| UninstallError::Run {
            program: step.program,
            source,
        })?;
        if !ok && !step.may_fail {
            return Err(UninstallError::Step {
                program: step.program,
                args: step.args.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
