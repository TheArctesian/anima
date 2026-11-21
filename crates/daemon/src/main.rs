//! `anima-daemon`: wires the command line to the library and sets the exit code. No rules here.

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::io::IsTerminal as _;
use std::process::ExitCode;

use anima_daemon::cli::{self, CliError, CodeSource, Command};
use anima_daemon::config::home::Home;
#[cfg(unix)]
use anima_daemon::config::private_file;
use anima_daemon::hub::enrolment::Enrolment;
use anima_daemon::hub::{HubClient, TIMEOUT};
use anima_daemon::keys::{self, store::KeyStore};
use anima_daemon::readers::Platform;
use anima_daemon::runtime;
use anima_daemon::token_store::TokenStore;
use anima_daemon::units::install::Host;
use anima_daemon::units::step::{STEP_DEADLINE, run_step};
use anima_daemon::units::{self, plan::Target};
use zeroize::Zeroizing;

fn main() -> ExitCode {
    // The launcher of a model runtime (R-NEW-containment, R-NEW-cgroupns): it joins the cgroup
    // of the server, hides the cgroups above it and becomes the runtime; it never returns. It
    // runs before the async runtime starts, because a new namespace needs one thread.
    #[cfg(target_os = "linux")]
    {
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        if arguments.first().map(String::as_str)
            == Some(anima_daemon::model::tree::linux::LAUNCH_MODE)
        {
            anima_daemon::model::tree::linux::launch(&arguments[1..]);
        }
    }
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(daemon()),
        Err(error) => failure(&error),
    }
}

async fn daemon() -> ExitCode {
    #[cfg(windows)]
    let _ = anima_windows::console::attach_parent();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let command = match cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => return usage(&error),
    };
    if let Command::Holder { folder, limit } = &command {
        return holder(folder, *limit).await;
    }
    let home = match Home::locate(
        |name| std::env::var(name).ok(),
        Platform::current() == Platform::Macos,
    ) {
        Ok(home) => home,
        Err(error) => return failure(&error),
    };
    match command {
        Command::Run { hosted_hub } => run(&home, hosted_hub).await,
        Command::Threads => threads(home.dir().to_owned()).await,
        Command::Enrol { code, hub } => enrol(code, hub, &home).await,
        Command::Install { target } => install(target, &home),
        Command::Uninstall => uninstall(&home),
        Command::KeyRead { name } => key_read(&name, &home),
        Command::Holder { .. } => ExitCode::FAILURE,
        Command::KeyForget => match KeyStore::open(&home).forget() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => failure(&error),
        },
    }
}

/// `anima-daemon key read <name>`: the value to standard output, never to a terminal.
fn key_read(name: &str, home: &Home) -> ExitCode {
    let mut out = std::io::stdout().lock();
    let to_terminal = out.is_terminal();
    let asked = (name, chrono::Utc::now());
    match keys::read(&KeyStore::open(home), asked, (&mut out, to_terminal)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => failure(&error),
    }
}

async fn enrol(
    code: CodeSource,
    hub: anima_daemon::config::origin::Origin,
    home: &Home,
) -> ExitCode {
    let code = match code {
        CodeSource::Given(code) => {
            eprintln!("{}", cli::ARGV_WARNING);
            code
        }
        CodeSource::Stdin => match read_code() {
            Ok(code) => code,
            Err(error) => return usage(&error),
        },
    };
    let client = match HubClient::reaching(hub.reach(), &home.link_socket(), TIMEOUT) {
        Ok(client) => client,
        Err(error) => return failure(&error),
    };
    let store = TokenStore::open(home);
    let enrolment = Enrolment {
        hub: &client,
        origin: hub,
        home,
        store: &store,
        now: chrono::Utc::now(),
    };
    match enrolment.spend(code).await {
        Ok(name) => {
            // A new enrolment is a new identity: no key of an earlier one stays (R-147).
            if let Err(error) = KeyStore::open(home).forget() {
                return failure(&error);
            }
            eprintln!("Enrolled {name}. Start the daemon with `anima-daemon run`.");
            ExitCode::SUCCESS
        }
        Err(error) => failure(&error),
    }
}

fn install(target: Target, home: &Home) -> ExitCode {
    let program = match std::env::current_exe() {
        Ok(program) => program,
        Err(error) => return failure(&error),
    };
    let host = Host {
        platform: Platform::current(),
        #[cfg(unix)]
        uid: match private_file::running_user() {
            Ok(uid) => uid,
            Err(error) => return failure(&error),
        },
        program: &program,
        home,
        now: chrono::Utc::now(),
    };
    let env = |name: &str| std::env::var(name).ok();
    match units::install::install(target, &host, env, |step| run_step(step, STEP_DEADLINE)) {
        Ok(file) => {
            eprintln!("Installed {} and started the daemon.", file.display());
            ExitCode::SUCCESS
        }
        Err(error) => failure(&error),
    }
}

/// `anima-daemon uninstall` (HMD-056): the user unit out first, then the token and the key cache.
#[cfg(unix)]
fn uninstall(home: &Home) -> ExitCode {
    use anima_daemon::units::removal::{self, Leaving, UninstallError};
    let uid = match private_file::running_user() {
        Ok(uid) => uid,
        Err(error) => return failure(&error),
    };
    let user = Leaving {
        platform: Platform::current(),
        uid,
    };
    let env = |name: &str| std::env::var(name).ok();
    let forget = || -> Result<(), UninstallError> {
        TokenStore::open(home).forget()?;
        KeyStore::open(home).forget()?;
        Ok(())
    };
    match removal::uninstall(user, env, |step| run_step(step, STEP_DEADLINE), forget) {
        Ok(done) if done.was_installed => {
            eprintln!("Removed {} and the daemon's token.", done.file.display());
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("No unit of the daemon was installed; its token is gone.");
            ExitCode::SUCCESS
        }
        Err(error) => failure(&error),
    }
}

/// The task of the daemon on Windows is not taken out by this command yet.
#[cfg(windows)]
fn uninstall(_home: &Home) -> ExitCode {
    eprintln!("`anima-daemon uninstall` is not built for Windows yet");
    ExitCode::FAILURE
}

/// The enrolment code from standard input; with echo off when it is a terminal. The code also
/// stays in `rpassword`'s buffer (terminal) or the `StdinLock`'s buffer (pipe), neither of which
/// is wiped (LOG-007); the process is short-lived.
fn read_code() -> Result<daemon_core::wire::EnrolmentCode, CliError> {
    if std::io::stdin().is_terminal() {
        eprint!("Enrolment code: ");
        let line = Zeroizing::new(rpassword::read_password().map_err(|_| CliError::NotACode)?);
        cli::code_from_line(&line)
    } else {
        cli::read_code(std::io::stdin().lock())
    }
}

/// Completes on Ctrl-C or SIGTERM.
#[cfg(unix)]
async fn stop() {
    use tokio::signal::unix::{SignalKind, signal};
    let Ok(mut terminate) = signal(SignalKind::terminate()) else {
        tokio::signal::ctrl_c().await.ok();
        return;
    };
    tokio::select! {
        _ = terminate.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
}

fn usage(error: &CliError) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(2)
}

fn failure(error: &dyn std::error::Error) -> ExitCode {
    eprintln!("{error}");
    ExitCode::FAILURE
}

#[cfg(windows)]
async fn stop() {
    let _ = tokio::signal::ctrl_c().await;
}

/// The daemon, and beside it the runner of the agent threads. The runner ends with the daemon;
/// its programs stay in the holder. The system unit and a hosted hub run as an account of their
/// own and start no agent (THR-128).
async fn run(home: &Home, hosted_hub: Option<std::path::PathBuf>) -> ExitCode {
    let runner = (hosted_hub.is_none() && serves_a_person(home.dir()))
        .then(|| tokio::spawn(threads(home.dir().to_owned())));
    let outcome = match runtime::run(home, hosted_hub, stop()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => failure(&error),
    };
    if let Some(runner) = runner {
        runner.abort();
    }
    outcome
}

/// The runner of the agent threads (TH11), until `SIGTERM` or Ctrl-C.
#[cfg(unix)]
async fn threads(daemon_dir: std::path::PathBuf) -> ExitCode {
    use anima_daemon::threads::runner;
    let program = match std::env::current_exe() {
        Ok(program) => program,
        Err(error) => return failure(&error),
    };
    // A name that is not UTF-8 is left out. A value is read as the login shell's output is
    // (lossy), so that a secret value that is not UTF-8 is still found and stripped (THR-040).
    let daemon: Vec<(String, String)> = std::env::vars_os()
        .filter_map(|(name, value)| {
            Some((
                name.into_string().ok()?,
                value.to_string_lossy().into_owned(),
            ))
        })
        .collect();
    match runner::run(&daemon_dir, &program, daemon, stop()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => failure(&error),
    }
}

#[cfg(unix)]
fn serves_a_person(daemon_dir: &std::path::Path) -> bool {
    anima_daemon::threads::runner::serves_a_person(daemon_dir)
}

#[cfg(not(unix))]
const fn serves_a_person(_: &std::path::Path) -> bool {
    false
}

#[cfg(not(unix))]
async fn threads(_: std::path::PathBuf) -> ExitCode {
    ExitCode::FAILURE
}

/// `anima-daemon holder <folder>`: the holder of the agent programs (THR-113).
#[cfg(unix)]
async fn holder(folder: &std::path::Path, limit: usize) -> ExitCode {
    match anima_daemon::threads::holder::hold(folder, limit).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => failure(&error),
    }
}

#[cfg(not(unix))]
async fn holder(_: &std::path::Path, _: usize) -> ExitCode {
    ExitCode::FAILURE
}
