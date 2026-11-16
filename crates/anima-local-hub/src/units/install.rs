//! `anima-local-hub install`, `remove`, `start` and `stop`: the copy of the programs, the unit
//! file, and the commands that start and stop the unit. The commands run through a function the caller
//! gives, so a test runs none of them.

use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::net::SocketAddr;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anima_database::{Platform, owner_only};

use super::steps::Step;
use super::trust;
use super::unit_file::{self, LABEL, Launch, RenderError, SYSTEMD_UNIT};
use crate::members::{Carried, LOCAL_HUB};
use crate::programs::{self, CopyError};
use crate::{Folders, run, stop_marker};

#[cfg(test)]
mod tests;

/// `launchctl bootstrap` right after `bootout` can fail once.
const BOOTSTRAP_RETRY: Duration = Duration::from_secs(1);

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error(transparent)]
    Copy(#[from] CopyError),
    #[error(transparent)]
    Render(#[from] RenderError),
    /// Another user could put a program of their own where the unit runs one at each login.
    #[error("{} can be changed by other users: run `chmod go-w` on it", .0.display())]
    OpenToOthers(PathBuf),
    #[error("cannot write {} ({kind})", path.display())]
    Write { path: PathBuf, kind: io::ErrorKind },
    #[error("`{program} {}` failed", args.join(" "))]
    Step {
        program: &'static str,
        args: Vec<String>,
    },
    #[error("`{program}` cannot be run ({kind})")]
    Run {
        program: &'static str,
        kind: io::ErrorKind,
    },
    #[error("the hub of this folder still runs; stop it first")]
    StillRunning,
    #[error("the hub is not installed; install it first")]
    NotInstalled,
}

/// Whose unit, and for which hub.
#[derive(Debug, Clone)]
pub struct Request<'a> {
    pub platform: Platform,
    pub uid: u32,
    pub home: &'a Path,
    pub folders: &'a Folders,
    /// Where the hub listens when it is not the default. S5 owns this choice.
    pub listen: Option<SocketAddr>,
    /// How long the running hub may take to stop after the unit was told to stop it.
    pub stop_limit: Duration,
}

impl Request<'_> {
    fn unit_file(&self) -> PathBuf {
        unit_file::file(self.platform, self.home)
    }

    fn launch(&self) -> Launch {
        let database = self.folders.database();
        let mut args = vec!["run".to_owned(), "--root".to_owned()];
        args.push(self.folders.root().to_string_lossy().into_owned());
        args.push("--socket-folder".to_owned());
        args.push(database.socket_folder().to_string_lossy().into_owned());
        if let Some(listen) = self.listen {
            args.extend(["--listen".to_owned(), listen.to_string()]);
        }
        Launch {
            program: Carried::installed_in(&self.folders.programs()).program(LOCAL_HUB),
            args,
        }
    }

    fn stop(&self) -> Step {
        match self.platform {
            Platform::MacOs => Step::launchctl(&["bootout", &format!("gui/{}/{LABEL}", self.uid)]),
            Platform::Linux => Step::systemctl(&["stop", SYSTEMD_UNIT]),
        }
        .may_fail()
    }

    fn start(&self, file: &Path) -> Vec<Step> {
        match self.platform {
            Platform::MacOs => vec![
                Step::launchctl(&[
                    "bootstrap",
                    &format!("gui/{}", self.uid),
                    &file.to_string_lossy(),
                ])
                .retried_after(BOOTSTRAP_RETRY),
            ],
            Platform::Linux => vec![
                Step::systemctl(&["daemon-reload"]),
                Step::systemctl(&["enable", SYSTEMD_UNIT]),
                Step::systemctl(&["restart", SYSTEMD_UNIT]),
            ],
        }
    }

    /// Succeeds when launchd has the unit loaded.
    fn loaded(&self) -> Step {
        Step::launchctl(&["print", &format!("gui/{}/{LABEL}", self.uid)])
    }

    /// Starts the unit that `stop` stopped; it stays enabled for the next login.
    fn start_again(&self, file: &Path) -> Vec<Step> {
        match self.platform {
            Platform::MacOs => self.start(file),
            Platform::Linux => vec![Step::systemctl(&["start", SYSTEMD_UNIT])],
        }
    }

    /// The unit does not start at the next login, and is stopped now.
    fn retire(&self) -> Step {
        match self.platform {
            Platform::MacOs => self.stop(),
            Platform::Linux => Step::systemctl(&["disable", "--now", SYSTEMD_UNIT]).may_fail(),
        }
    }
}

/// Installs or updates the hub of `request`: copies `source`, stops a hub that runs, puts the
/// copy in place, writes the unit file and starts the unit. Returns the unit file.
///
/// A fault before the stop leaves a running hub as it was.
///
/// # Errors
/// When the copy fails, the unit file cannot be written, a command fails, or the running hub
/// does not stop.
pub fn install(
    request: &Request<'_>,
    source: &Carried,
    run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<PathBuf, InstallError> {
    replace(request, source, run, Then::Start)
}

/// What the unit does after the programs were replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Then {
    Start,
    /// The operator stopped the hub: it stays stopped until `start` or the next login.
    StayStopped,
}

/// The steps of [`install`]: copy, stop, wait, put in place, write the unit file, and start
/// the unit when `then` says so. A start takes the operator's stop mark away.
pub(super) fn replace(
    request: &Request<'_>,
    source: &Carried,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
    then: Then,
) -> Result<PathBuf, InstallError> {
    let file = request.unit_file();
    let text = unit_file::render(request.platform, &request.launch())?;
    let root = request.folders.root();
    owner_only::folder(root).map_err(write_error(root))?;
    private(root, request.uid)?;
    programs::copy(source, request.folders)?;
    run_steps(&[request.stop()], &mut run)?;
    wait_until_stopped(request)?;
    programs::put_in_place(request.folders)?;
    write_unit(&file, &text, request.uid)?;
    if then == Then::Start {
        clear_stop_mark(request)?;
        run_steps(&request.start(&file), &mut run)?;
    } else if request.platform == Platform::Linux {
        // systemd reads the new unit file now, so a later `start` runs what it names.
        run_steps(&[Step::systemctl(&["daemon-reload"]).may_fail()], &mut run)?;
    }
    Ok(file)
}

/// Stops the unit and removes the unit file and the copied programs. The database, the key
/// and the logs stay.
///
/// # Errors
/// When the running hub does not stop, or a file cannot be removed.
pub fn remove(
    request: &Request<'_>,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<(), InstallError> {
    let file = request.unit_file();
    clear_stop_mark(request)?;
    run_steps(&[request.retire()], &mut run)?;
    wait_until_stopped(request)?;
    match fs::remove_file(&file) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(write_error(&file)(error)),
    }
    if request.platform == Platform::Linux {
        run_steps(&[Step::systemctl(&["daemon-reload"]).may_fail()], &mut run)?;
    }
    programs::remove(request.folders)?;
    Ok(())
}

/// Stops the unit now and waits until the hub stopped. The unit stays installed: it starts
/// again at the next login or at [`start`] (FRN-006).
///
/// # Errors
/// When the unit tool cannot be run, or the hub does not stop in the request's limit.
pub fn stop(
    request: &Request<'_>,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<(), InstallError> {
    // The mark first: a stop that fails half way is still the operator's.
    stop_marker::set(request.folders)
        .map_err(write_error(&request.folders.stopped_by_operator()))?;
    // A unit that is not loaded is stopped already.
    run_steps(&[request.stop()], &mut run)?;
    wait_until_stopped(request)
}

/// Starts the installed unit. A unit that launchd has loaded already is left as it is.
///
/// # Errors
/// [`InstallError::NotInstalled`] when there is no unit to start, or when a command fails.
pub fn start(
    request: &Request<'_>,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<(), InstallError> {
    if !is_installed(request) {
        return Err(InstallError::NotInstalled);
    }
    clear_stop_mark(request)?;
    if request.platform == Platform::MacOs {
        let loaded = request.loaded();
        if run(&loaded).map_err(|error| InstallError::Run {
            program: loaded.program,
            kind: error.kind(),
        })? {
            return Ok(());
        }
    }
    run_steps(&request.start_again(&request.unit_file()), &mut run)
}

/// Whether the unit file and the copied programs are there.
#[must_use]
pub fn is_installed(request: &Request<'_>) -> bool {
    request.unit_file().is_file() && programs::are_in_place(request.folders)
}

fn clear_stop_mark(request: &Request<'_>) -> Result<(), InstallError> {
    stop_marker::clear(request.folders).map_err(write_error(&request.folders.stopped_by_operator()))
}

fn run_steps(
    steps: &[Step],
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<(), InstallError> {
    for step in steps {
        let done = run(step).map_err(|error| InstallError::Run {
            program: step.program,
            kind: error.kind(),
        })?;
        if !done && !step.may_fail {
            return Err(InstallError::Step {
                program: step.program,
                args: step.args.clone(),
            });
        }
    }
    Ok(())
}

/// The lock of the hub root is the truth: the unit tool can return before the group stopped.
fn wait_until_stopped(request: &Request<'_>) -> Result<(), InstallError> {
    let started = Instant::now();
    while run::is_running(request.folders) {
        if started.elapsed() > request.stop_limit {
            return Err(InstallError::StillRunning);
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn write_error(path: &Path) -> impl FnOnce(io::Error) -> InstallError + '_ {
    move |error| InstallError::Write {
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}

/// Refuses a folder that another user can change, or that has such a folder above it.
fn private(folder: &Path, uid: u32) -> Result<(), InstallError> {
    match trust::open_to_others(folder, uid).map_err(write_error(folder))? {
        Some(open) => Err(InstallError::OpenToOthers(open)),
        None => Ok(()),
    }
}

/// Writes the unit file whole, in a folder that only this user (or root) can change: the
/// unit tool runs what the file names at each login.
fn write_unit(file: &Path, text: &str, uid: u32) -> Result<(), InstallError> {
    let folder = file.parent().unwrap_or(file);
    fs::create_dir_all(folder).map_err(write_error(folder))?;
    private(folder, uid)?;
    let mut being_written = file.as_os_str().to_owned();
    being_written.push(".new");
    let being_written = Path::new(&being_written);
    match fs::remove_file(being_written) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(write_error(being_written)(error)),
    }
    let mut new = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(being_written)
        .map_err(write_error(being_written))?;
    new.write_all(text.as_bytes())
        .and_then(|()| new.sync_all())
        .map_err(write_error(being_written))?;
    fs::rename(being_written, file).map_err(write_error(file))
}
