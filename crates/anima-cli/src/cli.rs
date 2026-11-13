//! The command tree. A new noun adds one variant to [`Command`] and one arm to
//! [`Command::run`], beside its `mod` line in `lib.rs`; nothing else here changes.

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser};

use crate::environment::Environment;
use crate::failure::{Failure, exit};
use crate::invocation::{Globals, Invocation};
use crate::output::Output;
use crate::{api, auth, builders, checks, contexts, devices, maintenance, releases, vpn};

/// The command line for an Anima hub.
#[derive(Debug, Parser)]
#[command(name = "anima", version, about, propagate_version = true)]
pub struct Cli {
    #[command(flatten)]
    globals: Globals,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Sign in and out (login, logout, status).
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    /// Named hubs (list, add, use, remove).
    Context {
        #[command(subcommand)]
        command: contexts::ContextCommand,
    },
    /// Devices (list, get).
    Device {
        #[command(subcommand)]
        command: devices::DeviceCommand,
    },
    /// Releases of the desktop application (list, next-version, create, cancel, log).
    Release {
        #[command(subcommand)]
        command: releases::ReleaseCommand,
    },
    /// Release builders (list, enrol, revoke).
    Builder {
        #[command(subcommand)]
        command: builders::BuilderCommand,
    },
    /// The VPN (status, invite, router-key, register, renew, approve-route).
    Vpn {
        #[command(subcommand)]
        command: vpn::VpnCommand,
    },
    /// Checks (list, get, create, update, delete, pause, resume, heartbeats, defaults, notifications).
    Check {
        #[command(subcommand)]
        command: checks::CheckCommand,
    },
    /// Maintenance windows (list, create, update, delete, start, stop).
    Maintenance {
        #[command(subcommand)]
        command: maintenance::MaintenanceCommand,
    },
    /// Send a GraphQL document and print the hub's answer.
    Api(api::ApiArgs),
    /// Print shell completion.
    Completion { shell: clap_complete::Shell },
}

impl Command {
    async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Auth { command } => command.run(invocation).await,
            Self::Context { command } => command.run(invocation),
            Self::Device { command } => command.run(invocation).await,
            Self::Release { command } => command.run(invocation).await,
            Self::Builder { command } => command.run(invocation).await,
            Self::Vpn { command } => command.run(invocation).await,
            Self::Check { command } => command.run(invocation).await,
            Self::Maintenance { command } => command.run(invocation).await,
            Self::Api(args) => args.run(invocation).await,
            Self::Completion { shell } => {
                let mut command = Cli::command();
                clap_complete::generate(shell, &mut command, "anima", &mut std::io::stdout());
                Ok(())
            }
        }
    }
}

/// Parses `args`, runs the command and returns the exit code (CMD-021).
pub async fn run(args: Vec<OsString>, environment: Environment) -> ExitCode {
    let json = args.iter().any(|arg| arg == "--json");
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(error) => return usage(&error, json, &environment),
    };
    let invocation = Invocation::new(environment, cli.globals);
    match cli.command.run(&invocation).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            invocation.output.failure(&failure);
            ExitCode::from(failure.exit_code())
        }
    }
}

/// Help and the version print as clap prints them; a usage error is exit 2 (CMD-021), as JSON
/// with `--json` (CMD-022).
fn usage(error: &clap::Error, json: bool, environment: &Environment) -> ExitCode {
    if !error.use_stderr() {
        let _ = error.print();
        return ExitCode::SUCCESS;
    }
    if !json {
        let _ = error.print();
        return ExitCode::from(exit::USAGE);
    }
    let rendered = error.render().to_string();
    let first = rendered.lines().next().unwrap_or_default();
    let failure = Failure::Usage {
        message: first.trim_start_matches("error: ").to_owned(),
        hint: Some("anima --help".to_owned()),
    };
    Output::new(true, environment.terminals.stdout, true).failure(&failure);
    ExitCode::from(exit::USAGE)
}
