//! Writes the unit file and starts it (MET-019, D-10): the only I/O of `anima-agent install`.
//! The decisions are in [`super::plan`] and [`super::render`].

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::plan::{self, ACCOUNT, Plan, PlanError, SYSTEM_HOME, Step, Target};
use super::render::{self, Program, RenderError};
use crate::config::home::Home;
use crate::readers::Platform;
use crate::token_store::{TokenStore, TokenStoreError};

/// A unit file is public text: the operator and the system manager read it.
const UNIT_MODE: u32 = 0o644;

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("{0}")]
    Plan(#[from] PlanError),
    #[error("{0}")]
    Program(#[from] RenderError),
    #[error("the agent's own file: {0}")]
    Binary(io::Error),
    #[error("{} is writable by other users, so the agent could be swapped: move it", .0.display())]
    UntrustedFolder(PathBuf),
    #[error(
        "this agent is not enrolled: run `anima-agent enrol -` (the code on standard input) first"
    )]
    NotEnrolled,
    #[error(
        "the system unit has no token: as the {ACCOUNT} account run `install -d -m 0700 -o {ACCOUNT} -g {ACCOUNT} {SYSTEM_HOME}`, then `printf '%s\\n' '<code>' | sudo -u {ACCOUNT} env ANIMA_AGENT_HOME={SYSTEM_HOME} anima-agent enrol -`"
    )]
    SystemNotEnrolled,
    #[error(
        "the system unit runs as the account {ACCOUNT}, which does not exist: run `useradd --system --user-group --no-create-home --home-dir {SYSTEM_HOME} --shell /usr/sbin/nologin {ACCOUNT}`, then install again"
    )]
    MissingAccount,
    #[error("the token store: {0}")]
    Token(#[from] TokenStoreError),
    #[error("writing {}: {source}", path.display())]
    Write { path: PathBuf, source: io::Error },
    #[error("{program} {} did not succeed", args.join(" "))]
    Step {
        program: &'static str,
        args: Vec<String>,
    },
    #[error("{program} could not be run: {source}")]
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
    /// The agent's own folder (where `enrol` stored the token).
    pub home: &'a Home,
}

/// Writes the unit for `target` and starts it, running each command through `run`
/// (`Ok(false)` is a command that exited with a failure). Returns the unit file's path.
///
/// Nothing is written until the binary's path, the enrolment and (for the system unit) the
/// account are all in order. An existing unit file is replaced whole, so a second run ends the
/// same as the first.
///
/// # Errors
/// [`InstallError`] for the first thing that is not in order or does not succeed.
pub fn install(
    target: Target,
    host: &Host<'_>,
    env: impl Fn(&str) -> Option<String>,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<PathBuf, InstallError> {
    let plan = plan::plan(target, host.platform, host.uid, env)?;
    let program = host.program.canonicalize().map_err(InstallError::Binary)?;
    check_folders(target, &program)?;
    let program = Program::parse(&program)?;
    let text = render::render(plan.unit, &program)?;
    check_ready(target, host.home)?;
    replace_file(&plan, &text)?;
    for step in &plan.steps {
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
    Ok(plan.file)
}

/// Every folder from the binary up to `/` is checked, for the parts a user can replace.
fn check_folders(target: Target, program: &Path) -> Result<(), InstallError> {
    for folder in program.ancestors().skip(1) {
        let metadata = fs::metadata(folder).map_err(InstallError::Binary)?;
        if !plan::folder_trusted(target, metadata.mode(), metadata.uid()) {
            return Err(InstallError::UntrustedFolder(folder.to_owned()));
        }
    }
    Ok(())
}

/// The unit must never start an agent that has no token: it would fail and restart for ever.
fn check_ready(target: Target, home: &Home) -> Result<(), InstallError> {
    match target {
        Target::User => {
            if TokenStore::open(home).load()?.is_none() {
                return Err(InstallError::NotEnrolled);
            }
        }
        Target::System => {
            // Root cannot open the account's 0600 token as its own, so only its presence counts.
            let passwd = fs::read_to_string("/etc/passwd").map_err(InstallError::Binary)?;
            if !render::has_account(&passwd, ACCOUNT) {
                return Err(InstallError::MissingAccount);
            }
            if !Home::at(PathBuf::from(SYSTEM_HOME)).token_path().is_file() {
                return Err(InstallError::SystemNotEnrolled);
            }
        }
    }
    Ok(())
}

/// Writes `plan.file` through a temporary file in its folder, renamed over the old one.
fn replace_file(plan: &Plan, text: &str) -> Result<(), InstallError> {
    let write = |source| InstallError::Write {
        path: plan.file.clone(),
        source,
    };
    let dir = plan.file.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir).map_err(write)?;
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
        .persist(&plan.file)
        .map_err(|error| write(error.error))?;
    File::open(dir).and_then(|d| d.sync_all()).map_err(write)
}

/// Runs a fixed command: an absolute program, no shell, no input, and only the variables the
/// session bus needs (`systemctl --user` finds the user's manager through them).
///
/// # Errors
/// The I/O error when the program cannot start; `Ok(false)` for a failure exit.
pub fn run_step(step: &Step) -> io::Result<bool> {
    let mut command = Command::new(step.program);
    command
        .args(&step.args)
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
        .stdin(Stdio::null());
    for name in ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    Ok(command.status()?.success())
}
