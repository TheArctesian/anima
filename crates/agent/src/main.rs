//! `anima-agent`: wires the command line to the library and sets the exit code. No rules here.

use std::io::Read as _;
use std::process::ExitCode;

use anima_agent::cli::{self, CliError, CodeSource, Command};
use anima_agent::config::home::Home;
use anima_agent::hub::HubClient;
use anima_agent::hub::enrolment::Enrolment;
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
        CodeSource::Given(code) => code,
        CodeSource::Stdin => match read_code() {
            Ok(code) => code,
            Err(error) => return usage(&error),
        },
    };
    let client = match HubClient::new(hub.url().clone()) {
        Ok(client) => client,
        Err(error) => return failure(&error),
    };
    let store = TokenStore::open(home);
    let enrolment = Enrolment {
        hub: &client,
        origin: hub,
        home,
        store: &store,
    };
    match enrolment.spend(code).await {
        Ok(name) => {
            eprintln!("Enrolled {name}. Start the agent with `anima-agent run`.");
            ExitCode::SUCCESS
        }
        Err(error) => failure(&error),
    }
}

/// The enrolment code from the first line of standard input.
fn read_code() -> Result<agent_core::wire::EnrolmentCode, CliError> {
    let mut input = Zeroizing::new(String::new());
    std::io::stdin()
        .take(256)
        .read_to_string(&mut input)
        .map_err(|_| CliError::NotACode)?;
    let line = input.lines().next().unwrap_or_default();
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
