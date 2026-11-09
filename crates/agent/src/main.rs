//! `anima-agent`: wires the command line to the library and sets the exit code. No rules here.

use std::io::{BufRead as _, IsTerminal as _, Read as _};
use std::process::ExitCode;

use anima_agent::cli::{self, CliError, CodeSource, Command};
use anima_agent::config::home::Home;
use anima_agent::hub::enrolment::Enrolment;
use anima_agent::hub::{HubClient, TIMEOUT};
use anima_agent::readers::Platform;
use anima_agent::runtime;
use anima_agent::token_store::TokenStore;
use zeroize::Zeroizing;

#[tokio::main]
async fn main() -> ExitCode {
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
    let home = match Home::locate(
        |name| std::env::var(name).ok(),
        Platform::current() == Platform::Macos,
    ) {
        Ok(home) => home,
        Err(error) => return failure(&error),
    };
    match command {
        Command::Run => match runtime::run(&home, stop()).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => failure(&error),
        },
        Command::Enrol { code, hub } => enrol(code, hub, &home).await,
    }
}

async fn enrol(
    code: CodeSource,
    hub: anima_agent::config::origin::Origin,
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
    let client = match HubClient::new(hub.url().clone(), TIMEOUT) {
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
            eprintln!("Enrolled {name}. Start the agent with `anima-agent run`.");
            ExitCode::SUCCESS
        }
        Err(error) => failure(&error),
    }
}

/// The enrolment code from one line of standard input; with echo off when it is a terminal.
fn read_code() -> Result<agent_core::wire::EnrolmentCode, CliError> {
    let line = if std::io::stdin().is_terminal() {
        eprint!("Enrolment code: ");
        Zeroizing::new(rpassword::read_password().map_err(|_| CliError::NotACode)?)
    } else {
        let mut input = Zeroizing::new(String::with_capacity(257));
        std::io::stdin()
            .lock()
            .take(256)
            .read_line(&mut input)
            .map_err(|_| CliError::NotACode)?;
        input
    };
    agent_core::wire::EnrolmentCode::parse(line.trim()).ok_or(CliError::NotACode)
}

/// Completes on Ctrl-C or SIGTERM.
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
