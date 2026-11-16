//! `anima-local-hub`: wires the command line to the library and sets the exit code. No rules here.
//!
//!   anima-local-hub run     [--root <folder> --socket-folder <folder>] [--listen <address:port>]
//!   anima-local-hub install --tree <folder> [--root ... --socket-folder ...] [--listen ...]
//!   anima-local-hub remove  [--root <folder> --socket-folder <folder>]
//!   anima-local-hub status  [--root <folder> --socket-folder <folder>]
//!   anima-local-hub start | stop  [--root <folder> --socket-folder <folder>]
//!   anima-local-hub account status | totp-new <name> | create <name>  [--root ... --socket-folder ...]
//!
//! `install` runs from the application bundle: it copies the programs beside it and the
//! PostgreSQL tree to the hub's folder and makes the unit that runs `run` at login.
//!
//! `run` prints `ready` on standard output when the hub serves, and stops the group in order
//! on SIGTERM or SIGINT. Nothing it prints holds a secret (LOG-007).

use std::io::Write as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};
use std::time::SystemTime;

use anima_database::{LayoutError, Platform, UserFolders};
use anima_local_hub::account::{self, AccountError, Action, Name};
use anima_local_hub::group::{LIMITS, Plan};
use anima_local_hub::run::{self, Ended, RunError};
use anima_local_hub::state::{self, Status};
use anima_local_hub::units::install::{self, Request};
use anima_local_hub::units::{STEP_LIMIT, run_step};
use anima_local_hub::{Carried, Folders, Stop};
use clap::{Args, Parser, Subcommand};

/// Another copy runs the hub of this folder.
const EXIT_ANOTHER_COPY: u8 = 10;
/// `account`: the hub is not ready, so its database does not run.
const EXIT_NOT_READY: u8 = 20;

/// The hub's address when none is given: the default of K7. S5 owns this choice.
const LISTEN: &str = "127.0.0.1:8443";

#[derive(Parser)]
#[command(name = "anima-local-hub", about = "The Anima hub on this computer")]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the hub's programs in order and keep them up until SIGTERM
    Run {
        #[command(flatten)]
        place: Place,
        /// Where the hub listens; a loopback address
        #[arg(long, default_value = LISTEN)]
        listen: SocketAddr,
        #[command(flatten)]
        programs: Programs,
    },
    /// Copy the programs out of the application and start the hub at each login
    Install {
        #[command(flatten)]
        place: Place,
        /// The PostgreSQL tree that the application carries
        #[arg(long)]
        tree: PathBuf,
        /// Where the hub listens, when it is not the default
        #[arg(long)]
        listen: Option<SocketAddr>,
    },
    /// Stop the hub and remove its unit and programs; the database and the key stay
    Remove(Place),
    /// Print as JSON whether the hub is installed and runs, and its last state
    Status(Place),
    /// Start the installed hub
    Start(Place),
    /// Stop the hub until `start` or the next login; it stays installed
    Stop(Place),
    /// The operator account, through the installed hub; secrets pass on standard input
    Account {
        #[command(flatten)]
        place: Place,
        #[arg(value_enum)]
        verb: AccountVerb,
        /// The operator's name: for `totp-new` and `create`
        #[arg(value_parser = Name::parse)]
        name: Option<Name>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum AccountVerb {
    /// One JSON line: the operator's name and whether TOTP is set, or null
    Status,
    /// One JSON line with a new TOTP secret and its URI; nothing is saved
    TotpNew,
    /// Make or replace the operator: password, secret and code on three lines of input
    Create,
}

#[derive(Args)]
struct Place {
    /// The hub's folder; the default is the application's folder of this user
    #[arg(long, requires = "socket_folder")]
    root: Option<PathBuf>,
    /// The folder for the database's Unix socket
    #[arg(long, requires = "root")]
    socket_folder: Option<PathBuf>,
}

impl Place {
    fn folders(&self) -> Result<Folders, LayoutError> {
        match (&self.root, &self.socket_folder) {
            (Some(root), Some(socket_folder)) => Folders::new(root.clone(), socket_folder.clone()),
            _ => Folders::for_user(
                Platform::HOST,
                &UserFolders {
                    home: std::env::var_os("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_default(),
                    xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
                    xdg_runtime_dir: std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from),
                },
            ),
        }
    }
}

/// Where the programs are, when they are not around this program as `install` puts them.
#[derive(Args)]
struct Programs {
    /// The folder with anima-hub, anima-database and anima-gateway
    #[arg(long, requires = "tree")]
    programs: Option<PathBuf>,
    /// The PostgreSQL tree: a folder with bin/, lib/ and share/
    #[arg(long, requires = "programs")]
    tree: Option<PathBuf>,
}

impl Programs {
    fn carried(self) -> Result<Carried, Failure> {
        if let (Some(programs), Some(tree)) = (self.programs, self.tree) {
            return Ok(Carried::new(programs, tree));
        }
        Carried::around(&std::env::current_exe()?)
            .ok_or_else(|| "this program is not in a bin folder beside its PostgreSQL tree".into())
    }
}

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
        } => install(&place, tree, listen),
        Command::Remove(place) => remove(&place),
        Command::Status(place) => status(&place),
        Command::Start(place) => start_or_stop(&place, true),
        Command::Stop(place) => start_or_stop(&place, false),
        Command::Account { place, verb, name } => account(&place, verb, name),
    };
    match outcome {
        Ok(code) => code,
        Err(error) => {
            eprintln!("anima-local-hub: {error}");
            let code = if matches!(
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

fn serve(place: &Place, listen: SocketAddr, programs: Programs) -> Result<ExitCode, Failure> {
    let plan = Plan {
        carried: programs.carried()?,
        folders: place.folders()?,
        listen: loopback(listen)?,
        limits: LIMITS,
        clock: SystemTime::now,
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
        }
    }
}

fn install(place: &Place, tree: PathBuf, listen: Option<SocketAddr>) -> Result<ExitCode, Failure> {
    let (folders, user) = (place.folders()?, User::current()?);
    let listen = listen.map(loopback).transpose()?;
    let program = std::env::current_exe()?;
    let source = Carried::new(
        program.parent().unwrap_or(Path::new("/")).to_path_buf(),
        tree,
    );
    let file = install::install(&user.request(&folders, listen), &source, |step| {
        run_step(step, &user.session, STEP_LIMIT)
    })?;
    println!("installed: {}", file.display());
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
