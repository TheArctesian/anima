//! Writes the unit file and starts it (MET-019, D-10): the only I/O of `anima-agent install`.
//! The decisions are in [`super::plan`], [`super::ready`], [`super::render`] and
//! [`super::step`].

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::plan::{self, Plan, PlanError, Target};
use super::ready::{self, NotReady};
use super::render::{self, Program, RenderError, Unit};
use super::step::{self, NoSessionBus, Step};
use crate::config::home::Home;
use crate::readers::Platform;
use crate::token_store::{TokenStore, TokenStoreError};

/// A unit file is public text: the operator and the system manager read it.
const UNIT_MODE: u32 = 0o644;

/// Where the system unit's account is looked up.
const PASSWD: &str = "/etc/passwd";

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("{0}")]
    Plan(#[from] PlanError),
    #[error("{0}")]
    Program(#[from] RenderError),
    #[error("the agent's own file: {0}")]
    Binary(io::Error),
    #[error("{} is writable by others or not owned by root or this user, so the agent could be swapped: reinstall it", .0.display())]
    UntrustedBinary(PathBuf),
    #[error("{} is writable by other users, so the agent could be swapped: move it", .0.display())]
    UntrustedFolder(PathBuf),
    #[error("{0}")]
    NotReady(#[from] NotReady),
    #[error("{0}")]
    NoSessionBus(#[from] NoSessionBus),
    #[error("the token store: {0}")]
    Token(#[from] TokenStoreError),
    #[error("reading {}: {source}", path.display())]
    Read { path: PathBuf, source: io::Error },
    #[error("writing {}: {source}", path.display())]
    Write { path: PathBuf, source: io::Error },
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
}

/// This computer and this run: what `install` needs to know besides the target.
pub struct Host<'a> {
    pub platform: Platform,
    pub uid: u32,
    /// The running binary, as `std::env::current_exe` gives it.
    pub program: &'a Path,
    /// The agent's own folder as this shell finds it (where `enrol` stored the token).
    pub home: &'a Home,
    /// Now: a token that expired by then is no token.
    pub now: DateTime<Utc>,
}

/// Writes the unit for `target` and starts it, running each command through `run`
/// (`Ok(false)` is a command that exited with a failure). Returns the unit file's path.
///
/// Nothing is written until the binary and its folders, the enrolment, the agent's folder and
/// (for the system unit) the account are all in order; the unit's own folders are checked
/// before the file goes in. An existing unit file is replaced whole, so a second run ends the
/// same as the first.
///
/// # Errors
/// [`InstallError`] for the first thing that is not in order or does not succeed.
pub fn install(
    target: Target,
    host: &Host<'_>,
    env: impl Fn(&str) -> Option<String>,
    run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<PathBuf, InstallError> {
    let mut plan = plan::plan(target, host.platform, host.uid, &env)?;
    if plan.unit == Unit::SystemdUser {
        let bus = session_bus(&env, host.uid)?;
        for step in &mut plan.steps {
            step.env.clone_from(&bus);
        }
    }
    let program = host.program.canonicalize().map_err(InstallError::Binary)?;
    check_binary(target, host.uid, &program)?;
    let program = Program::parse(&program)?;
    let text = render::render(plan.unit, &program)?;
    check_ready(&plan, host, program.as_str())?;
    replace_file(&plan, target, host.uid, &text)?;
    run_steps(&plan.steps, run)?;
    Ok(plan.file)
}

/// The variables `systemctl --user` reaches the user's manager through.
fn session_bus(
    env: impl Fn(&str) -> Option<String>,
    uid: u32,
) -> Result<Vec<(&'static str, String)>, NoSessionBus> {
    let fallback = fs::metadata(step::fallback_session_folder(uid))
        .ok()
        .filter(fs::Metadata::is_dir)
        .map(|folder| folder.uid());
    let folder = step::session_folder(env("XDG_RUNTIME_DIR"), uid, fallback)?;
    let mut vars = vec![("XDG_RUNTIME_DIR", folder)];
    if let Some(address) = env("DBUS_SESSION_BUS_ADDRESS").filter(|a| !a.is_empty()) {
        vars.push(("DBUS_SESSION_BUS_ADDRESS", address));
    }
    Ok(vars)
}

/// The binary itself (by its real path) and every folder above it, for the parts another user
/// could replace.
fn check_binary(target: Target, uid: u32, program: &Path) -> Result<(), InstallError> {
    let metadata = fs::metadata(program).map_err(InstallError::Binary)?;
    if !plan::binary_trusted(target, metadata.mode(), metadata.uid(), uid) {
        return Err(InstallError::UntrustedBinary(program.to_owned()));
    }
    let folder = program.parent().unwrap_or_else(|| Path::new("/"));
    check_folders(target, uid, folder)
}

/// `folder` and every folder above it, as they stand: the caller passes a real path.
fn check_folders(target: Target, uid: u32, folder: &Path) -> Result<(), InstallError> {
    for folder in folder.ancestors() {
        let metadata = fs::metadata(folder).map_err(|source| InstallError::Read {
            path: folder.to_owned(),
            source,
        })?;
        if !plan::folder_trusted(target, metadata.mode(), metadata.uid(), uid) {
            return Err(InstallError::UntrustedFolder(folder.to_owned()));
        }
    }
    Ok(())
}

/// The unit must never start an agent that has no usable token: it would fail and restart for
/// ever.
fn check_ready(plan: &Plan, host: &Host<'_>, program: &str) -> Result<(), InstallError> {
    if plan.unit == Unit::SystemdSystem {
        let passwd = fs::read_to_string(PASSWD).map_err(|source| InstallError::Read {
            path: PathBuf::from(PASSWD),
            source,
        })?;
        let token = Home::at(plan.home.clone()).token_path();
        let found = match fs::metadata(&token) {
            Ok(file) if file.is_file() => Some((file.mode(), file.uid())),
            Ok(_) => None,
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(InstallError::Read {
                    path: token,
                    source,
                });
            }
        };
        ready::system(&passwd, found, program)?;
    } else {
        ready::same_home(host.home.dir(), &plan.home)?;
        let expires_at = TokenStore::open(host.home)
            .load()?
            .map(|credential| credential.expires_at);
        ready::token_usable(expires_at, host.now)?;
    }
    Ok(())
}

/// Writes `plan.file` through a temporary file in its folder, renamed over the old one. The
/// folder is made first, then checked by its real path, so a link to a folder others can write
/// never gets the unit.
fn replace_file(plan: &Plan, target: Target, uid: u32, text: &str) -> Result<(), InstallError> {
    let write = |source| InstallError::Write {
        path: plan.file.clone(),
        source,
    };
    let dir = plan.file.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir).map_err(write)?;
    let dir = dir.canonicalize().map_err(write)?;
    check_folders(target, uid, &dir)?;
    let name = plan.file.file_name().unwrap_or(plan.file.as_os_str());
    let mut temporary = tempfile::Builder::new()
        .prefix(".tmp-")
        .tempfile_in(&dir)
        .map_err(write)?;
    temporary.write_all(text.as_bytes()).map_err(write)?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(UNIT_MODE))
        .map_err(write)?;
    temporary.as_file().sync_all().map_err(write)?;
    temporary
        .persist(dir.join(name))
        .map_err(|error| write(error.error))?;
    File::open(&dir).and_then(|d| d.sync_all()).map_err(write)
}

/// Runs each step in order; a failure stops the rest unless the step may fail.
fn run_steps(
    steps: &[Step],
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<(), InstallError> {
    for step in steps {
        let ok = run(step).map_err(|source| InstallError::Run {
            program: step.program,
            source,
        })?;
        if !ok && !step.may_fail {
            return Err(InstallError::Step {
                program: step.program,
                args: step.args.clone(),
            });
        }
    }
    Ok(())
}
