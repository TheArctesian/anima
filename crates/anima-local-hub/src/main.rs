//! `anima-local-hub`: wires the command line to the library and sets the exit code. No rules here.
//!
//!   anima-local-hub run     [--root <folder> --socket-folder <folder>] [--listen <address:port>]
//!                           [--hosted-cloud-key-file <file>]
//!   anima-local-hub install --tree <folder> [--root ... --socket-folder ...] [--listen ...] [--step-back]
//!   anima-local-hub update  --tree <folder> [--root ... --socket-folder ...]
//!   anima-local-hub remove  [--root <folder> --socket-folder <folder>]
//!   anima-local-hub status  [--root <folder> --socket-folder <folder>]
//!   anima-local-hub start | stop  [--root <folder> --socket-folder <folder>]
//!   anima-local-hub account status | totp-new <name> | create <name>  [--root ... --socket-folder ...]
//!
//! `install` runs from the application bundle: it copies the programs beside it and the
//! PostgreSQL tree to the hub's folder and makes the unit that runs `run` at login.
//! `install` refuses programs of a lower version than the installed ones unless `--step-back`
//! is given; the application never gives it. `--listen` is a loopback address with a port.
//! `update` does the same only when the installed hub exists and its programs differ from the
//! ones that the application carries, and are not older; it prints `{"updated":true}` or
//! `{"updated":false}` (`{"updated":false,"older":true}` and a line on standard error when the programs are older). It keeps the `--listen`
//! that `install` was given.
//!
//! `run` prints `ready` on standard output when the hub serves, and stops the group in order
//! on SIGTERM or SIGINT. Nothing it prints holds a secret (LOG-007).

mod arguments;

use std::io::Write as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};
use std::time::SystemTime;

use anima_database::Platform;
use anima_local_hub::account::{self, AccountError, Action, Name};
use anima_local_hub::group::{LIMITS, Plan};
use anima_local_hub::programs;
use anima_local_hub::root_copy::{self, RootCopyError};
use anima_local_hub::run::{self, Ended, RunError};
use anima_local_hub::state::{self, Status};
use anima_local_hub::units::install::{self, Request};
use anima_local_hub::units::listen::require_allowed;
use anima_local_hub::units::update;
use anima_local_hub::units::{STEP_LIMIT, run_step};
use anima_local_hub::{Carried, Folders, Stop};
use clap::Parser as _;

use arguments::{AccountVerb, Arguments, Command, Place, RunFiles};

/// Another copy runs the hub of this folder.
const EXIT_ANOTHER_COPY: u8 = 10;
/// `account`: the hub is not ready, so its database does not run.
const EXIT_NOT_READY: u8 = 20;

/// The hub's address when none is given: the default of K7. S5 owns this choice.
pub(crate) const LISTEN: &str = "127.0.0.1:8443";

type Failure = Box<dyn std::error::Error>;

fn main() -> ExitCode {
    let outcome = match Arguments::parse().command {
        Command::Run {
            place,
            listen,
            programs,
        } => serve(&place, listen, programs),
        Command::Install {
            place,
            tree,
            listen,
            step_back,
        } => install(&place, tree, listen, step_back),
        Command::Update { place, tree } => update(&place, tree),
        Command::Remove(place) => remove(&place),
        Command::Status(place) => status(&place),
        Command::Start(place) => start_or_stop(&place, true),
        Command::Stop(place) => start_or_stop(&place, false),
        Command::Copy { place, tree, to } => copy(&place, tree, &to),
        Command::CheckCopy { copy, tree } => check_copy(&copy, &tree),
        Command::Account { place, verb, name } => account(&place, verb, name),
    };
    match outcome {
        Ok(code) => code,
        Err(error) => {
            eprintln!("anima-local-hub: {error}");
            let runs = matches!(
                error.downcast_ref::<RootCopyError>(),
                Some(RootCopyError::HubRuns)
            );
            let code = if runs
                || matches!(
                    error.downcast_ref::<RunError>(),
                    Some(RunError::AnotherCopy)
                ) {
                EXIT_ANOTHER_COPY
            } else if matches!(
                error.downcast_ref::<AccountError>(),
                Some(AccountError::NotReady)
            ) {
                EXIT_NOT_READY
            } else {
                1
            };
            ExitCode::from(code)
        }
    }
}

/// NET-003: without TLS the hub may listen on loopback only. The hub checks it too.
fn loopback(listen: SocketAddr) -> Result<SocketAddr, Failure> {
    if !listen.ip().is_loopback() {
        return Err("--listen must be a loopback address".into());
    }
    Ok(listen)
}

fn serve(place: &Place, listen: SocketAddr, programs: RunFiles) -> Result<ExitCode, Failure> {
    let hosted_cloud_key_file = programs.hosted_cloud_key_file.clone();
    let plan = Plan {
        carried: programs.carried()?,
        folders: place.folders()?,
        listen: loopback(listen)?,
        limits: LIMITS,
        clock: SystemTime::now,
        hosted_cloud_key_file,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let ended = runtime.block_on(async {
        // Before the first member starts: a signal that comes during the start is kept.
        let stop = Stop::on_signals()?;
        let ready = || {
            // Nobody may read it: a closed output must not end a hub that serves.
            let mut output = std::io::stdout();
            let _ = writeln!(output, "ready").and_then(|()| output.flush());
        };
        Ok::<_, Failure>(run::run(&plan, stop, ready).await?)
    })?;
    Ok(match ended {
        Ended::Asked => ExitCode::SUCCESS,
        Ended::Failed => ExitCode::FAILURE,
    })
}

/// The unit of the user who runs this program. `systemctl --user` finds the user's systemd
/// through the two session variables.
struct User {
    home: PathBuf,
    session: Vec<(&'static str, String)>,
}

impl User {
    fn current() -> Result<Self, Failure> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|home| home.is_absolute())
            .ok_or("HOME is not set to an absolute path")?;
        let session = ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"]
            .into_iter()
            .filter_map(|name| Some((name, std::env::var(name).ok()?)))
            .collect();
        Ok(Self { home, session })
    }

    fn request<'a>(&'a self, folders: &'a Folders, listen: Option<SocketAddr>) -> Request<'a> {
        Request {
            platform: Platform::HOST,
            uid: nix::unistd::Uid::effective().as_raw(),
            home: &self.home,
            folders,
            listen,
            stop_limit: STEP_LIMIT,
            version: programs::version::THIS,
            step_back: false,
        }
    }
}

/// What the application carries: the programs beside this one, and `tree`.
fn carried_by_application(tree: PathBuf) -> Result<Carried, Failure> {
    let program = std::env::current_exe()?;
    Ok(Carried::new(
        program.parent().unwrap_or(Path::new("/")).to_path_buf(),
        tree,
    ))
}

fn install(
    place: &Place,
    tree: PathBuf,
    listen: Option<SocketAddr>,
    step_back: bool,
) -> Result<ExitCode, Failure> {
    let (folders, user) = (place.folders()?, User::current()?);
    let listen = listen.map(require_allowed).transpose()?;
    let source = carried_by_application(tree)?;
    let request = Request {
        step_back,
        ..user.request(&folders, listen)
    };
    let file = install::install(&request, &source, |step| {
        run_step(step, &user.session, STEP_LIMIT)
    })?;
    println!("installed: {}", file.display());
    Ok(ExitCode::SUCCESS)
}

fn update(place: &Place, tree: PathBuf) -> Result<ExitCode, Failure> {
    let (folders, user) = (place.folders()?, User::current()?);
    let source = carried_by_application(tree)?;
    let outcome = update::update(&user.request(&folders, None), &source, |step| {
        run_step(step, &user.session, STEP_LIMIT)
    })?;
    if let Some(reason) = outcome.refusal() {
        eprintln!("anima-local-hub: not updated: {reason}");
    }
    println!("{}", outcome.json_line());
    Ok(ExitCode::SUCCESS)
}

fn remove(place: &Place) -> Result<ExitCode, Failure> {
    let (folders, user) = (place.folders()?, User::current()?);
    install::remove(&user.request(&folders, None), |step| {
        run_step(step, &user.session, STEP_LIMIT)
    })?;
    Ok(ExitCode::SUCCESS)
}

fn status(place: &Place) -> Result<ExitCode, Failure> {
    let (folders, user) = (place.folders()?, User::current()?);
    let status = Status {
        installed: install::is_installed(&user.request(&folders, None)),
        running: run::is_running(&folders),
        last: state::read(&folders.state_file()),
    };
    println!("{}", serde_json::to_string(&status)?);
    Ok(ExitCode::SUCCESS)
}

/// One JSON line: how many entries and bytes the copy holds. No path and no content.
fn copy(place: &Place, tree: PathBuf, to: &Path) -> Result<ExitCode, Failure> {
    let made = root_copy::make(&carried_by_application(tree)?, &place.folders()?, to)?;
    println!("{}", serde_json::to_string(&made)?);
    Ok(ExitCode::SUCCESS)
}

fn check_copy(copy: &Path, tree: &Path) -> Result<ExitCode, Failure> {
    let sound = root_copy::check(tree, copy)?;
    println!("{}", serde_json::to_string(&sound)?);
    Ok(ExitCode::SUCCESS)
}

fn start_or_stop(place: &Place, start: bool) -> Result<ExitCode, Failure> {
    let (folders, user) = (place.folders()?, User::current()?);
    let request = user.request(&folders, None);
    let run = |step: &_| run_step(step, &user.session, STEP_LIMIT);
    if start {
        install::start(&request, run)?;
    } else {
        install::stop(&request, run)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Standard input and output pass to the hub as they are: they carry the secrets. On a
/// failure only a fixed message is printed, never the hub's own text.
fn account(place: &Place, verb: AccountVerb, name: Option<Name>) -> Result<ExitCode, Failure> {
    let action = match (verb, name) {
        (AccountVerb::Status, None) => Action::Status,
        (AccountVerb::TotpNew, Some(name)) => Action::TotpNew(name),
        (AccountVerb::Create, Some(name)) => Action::Create(name),
        (AccountVerb::Status, Some(_)) => return Err("`account status` takes no name".into()),
        (_, None) => return Err("give the operator's name".into()),
    };
    let ended = account::run(
        &place.folders()?,
        &action,
        Stdio::inherit(),
        Stdio::inherit(),
    )?;
    if ended.success() {
        return Ok(ExitCode::SUCCESS);
    }
    let code = ended.code().and_then(|code| u8::try_from(code).ok());
    eprintln!(
        "anima-local-hub: the hub refused the account command (exit code {})",
        code.map_or_else(|| "none".to_owned(), |code| code.to_string())
    );
    Ok(ExitCode::from(code.filter(|&code| code != 0).unwrap_or(1)))
}
