//! Writes the unit file and starts it (MET-019, D-10): the only I/O of `anima-agent install`.
//! The decisions are in [`super::plan`], [`super::ready`], [`super::render`],
//! [`super::step`] and [`super::trust`]; the folders are made and checked in
//! [`super::folders`].

use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use zeroize::Zeroizing;

use super::folders::{self, FolderError};
use super::plan::{self, Plan, PlanError, Target};
use super::ready::{self, NotReady};
use super::render::{self, Program, RenderError, Unit};
use super::step::{self, NoSessionBus, Step};
use super::trust::{self, Installer};
use crate::config::home::Home;
use crate::readers::Platform;
use crate::token_store::{TokenStore, TokenStoreError};

/// A unit file is public text: the operator and the system manager read it.
const UNIT_MODE: u32 = 0o644;

/// Where accounts and groups are looked up.
const PASSWD: &str = "/etc/passwd";
const GROUP: &str = "/etc/group";

/// The most of the system token file that is read: a record is a few hundred bytes.
const RECORD_LIMIT: u64 = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("{0}")]
    Plan(#[from] PlanError),
    #[error("{0}")]
    Program(#[from] RenderError),
    #[error("the agent's own file: {0}")]
    Binary(io::Error),
    #[error(
        "{} is under {root}, which the unit's sandbox hides from the agent: move the binary out of {root} (for example to /usr/local/bin) and install again",
        program.display()
    )]
    Hidden {
        program: PathBuf,
        root: &'static str,
    },
    #[error("{0}")]
    Folder(#[from] FolderError),
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
    let passwd = read_text(Path::new(PASSWD))?;
    let by = Installer {
        target,
        uid: host.uid,
        private_group: trust::private_group(host.uid, &passwd, &read_text(Path::new(GROUP))?),
    };
    let program = host.program.canonicalize().map_err(InstallError::Binary)?;
    if let Some(root) = plan::hidden_root(plan.unit, &program) {
        return Err(InstallError::Hidden { program, root });
    }
    folders::check_binary(&program, by)?;
    let program = Program::parse(&program)?;
    let text = render::render(plan.unit, &program)?;
    if plan.unit == Unit::SystemdSystem {
        check_system_ready(&plan, &passwd, host.now, program.as_str())?;
    } else {
        check_user_ready(&plan, host)?;
    }
    let dir = plan.file.parent().unwrap_or_else(|| Path::new("."));
    let dir = folders::make_folder(dir, by)?;
    replace_file(&plan.file, &dir, &text)?;
    run_steps(&plan.steps, run)?;
    Ok(plan.file)
}

fn read_text(path: &Path) -> Result<String, InstallError> {
    fs::read_to_string(path).map_err(|source| InstallError::Read {
        path: path.to_owned(),
        source,
    })
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

/// The user unit's agent must find a token that has not expired, in the folder this shell
/// found: else it fails and restarts for ever.
fn check_user_ready(plan: &Plan, host: &Host<'_>) -> Result<(), InstallError> {
    ready::same_home(host.home.dir(), &plan.home)?;
    let expires_at = TokenStore::open(host.home)
        .load()?
        .map(|credential| credential.expires_at);
    ready::token_usable(expires_at, host.now)?;
    Ok(())
}

/// The system unit's account must exist and own its token, which must not have expired. Root
/// reads the token itself only after its owner and mode are checked, and never shows it.
fn check_system_ready(
    plan: &Plan,
    passwd: &str,
    now: DateTime<Utc>,
    program: &str,
) -> Result<(), InstallError> {
    let token = Home::at(plan.home.clone()).token_path();
    let read = |source| InstallError::Read {
        path: token.clone(),
        source,
    };
    // A link or a pipe is no token; checked before opening, so a pipe never blocks the open.
    let file = match fs::symlink_metadata(&token) {
        Ok(found) if found.is_file() => Some(File::open(&token).map_err(read)?),
        Ok(_) => None,
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(source) => return Err(read(source)),
    };
    let found = match &file {
        Some(file) => Some(file.metadata().map_err(read)?),
        None => None,
    };
    ready::system(passwd, found.as_ref().map(|m| (m.mode(), m.uid())), program)?;
    let Some(file) = file else {
        return Ok(());
    };
    let mut record = Zeroizing::new(Vec::new());
    file.take(RECORD_LIMIT)
        .read_to_end(&mut record)
        .map_err(read)?;
    ready::system_token(&record, now, program)?;
    Ok(())
}

/// Writes `file` as `name` in `dir` (its folder's checked real path) through a temporary file
/// there, renamed over the old one.
fn replace_file(file: &Path, dir: &Path, text: &str) -> Result<(), InstallError> {
    let write = |source| InstallError::Write {
        path: file.to_owned(),
        source,
    };
    let name = file.file_name().unwrap_or(file.as_os_str());
    let mut temporary = tempfile::Builder::new()
        .prefix(".tmp-")
        .tempfile_in(dir)
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
    File::open(dir).and_then(|d| d.sync_all()).map_err(write)
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
