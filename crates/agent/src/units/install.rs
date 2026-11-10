//! Writes the unit file and starts it (MET-019, D-10): the only I/O of `anima-agent install`.
//! The decisions are in [`super::plan`], [`super::ready`], [`super::render`],
//! [`super::sandbox`] and [`super::trust`]; the folders are made and checked in
//! [`super::folders`], the system token in [`super::token_file`], the session bus found in
//! [`super::session`] and the commands run by [`super::step`].

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::folders::{self, FolderError};
use super::plan::{self, Plan, PlanError, Target};
use super::ready::{self, NotReady};
use super::render::{self, Program, RenderError, Unit};
use super::sandbox;
use super::session::{self, NoSessionBus};
use super::step::Step;
use super::token_file::{self, SystemTokenError};
use super::trust::{self, Installer};
use crate::config::home::Home;
use crate::readers::Platform;
use crate::token_store::{TokenStore, TokenStoreError};

/// A unit file is public text: the operator and the system manager read it.
const UNIT_MODE: u32 = 0o644;

/// Where accounts and groups are looked up.
const PASSWD: &str = "/etc/passwd";
const GROUP: &str = "/etc/group";

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
    #[error("{0}")]
    SystemToken(#[from] SystemTokenError),
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
        let bus = session::session_bus(&env, host.uid)?;
        for step in &mut plan.steps {
            step.env.clone_from(&bus);
        }
    }
    let passwd = read_text(Path::new(PASSWD))?;
    // An unreadable /etc/group means no private group: group write is then refused.
    let group = fs::read_to_string(GROUP).ok();
    let by = Installer {
        target,
        uid: host.uid,
        private_group: trust::private_group(host.uid, &passwd, group.as_deref()),
    };
    let program = host.program.canonicalize().map_err(InstallError::Binary)?;
    if let Some(root) = sandbox::hidden_root(&hidden_roots(plan.unit)?, &program) {
        return Err(InstallError::Hidden { program, root });
    }
    folders::check_binary(&program, by)?;
    let program = Program::parse(&program)?;
    let text = render::render(plan.unit, &program)?;
    if plan.unit == Unit::SystemdSystem {
        let token = Home::at(plan.home.clone()).token_path();
        token_file::check(&token, &passwd, host.now, program.as_str())?;
    } else {
        check_user_ready(&plan, host)?;
    }
    let dir = plan.file.parent().unwrap_or_else(|| Path::new("."));
    let dir = folders::make_folder(dir, by)?;
    replace_file(&plan.file, &dir, &text)?;
    run_steps(&plan.steps, run)?;
    Ok(plan.file)
}

/// The folders `unit`'s sandbox hides, each with its real path (`None` when it does not exist).
fn hidden_roots(unit: Unit) -> Result<Vec<(&'static str, Option<PathBuf>)>, InstallError> {
    let mut roots = Vec::new();
    for root in sandbox::hidden_roots(unit) {
        let real = match Path::new(root).canonicalize() {
            Ok(real) => Some(real),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(InstallError::Read {
                    path: PathBuf::from(root),
                    source,
                });
            }
        };
        roots.push((*root, real));
    }
    Ok(roots)
}

fn read_text(path: &Path) -> Result<String, InstallError> {
    fs::read_to_string(path).map_err(|source| InstallError::Read {
        path: path.to_owned(),
        source,
    })
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
pub(super) fn run_steps(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::plan::plan;

    fn macos() -> Vec<Step> {
        let env = |name: &str| (name == "HOME").then(|| "/Users/ada".to_owned());
        plan(Target::User, Platform::Macos, 501, env)
            .expect("a plan")
            .steps
    }

    /// Runs the macOS steps, `bootout` and `bootstrap` answering as given; returns the
    /// result and the verbs that ran.
    fn run_macos(bootout: bool, bootstrap: bool) -> (Result<(), InstallError>, Vec<String>) {
        let mut ran = Vec::new();
        let result = run_steps(&macos(), |step| {
            ran.push(step.args[0].clone());
            Ok(if step.args[0] == "bootout" {
                bootout
            } else {
                bootstrap
            })
        });
        (result, ran)
    }

    #[test]
    fn met_019_a_first_macos_install_whose_bootout_fails_still_bootstraps() {
        let (result, ran) = run_macos(false, true);
        result.expect("bootout may fail");
        assert_eq!(ran, ["bootout", "bootstrap"]);
    }

    #[test]
    fn met_019_a_failed_macos_bootstrap_is_an_error_naming_it() {
        let (result, ran) = run_macos(true, false);
        let error = result.expect_err("bootstrap must succeed");
        assert!(
            matches!(&error, InstallError::Step { program: "/bin/launchctl", args } if args[0] == "bootstrap"),
            "{error}"
        );
        assert_eq!(ran, ["bootout", "bootstrap"]);
    }
}
