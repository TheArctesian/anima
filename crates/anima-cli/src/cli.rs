//! The command tree. A new noun adds one variant to [`Command`] and one arm to
//! [`Command::run`], beside its `mod` line in `lib.rs`; nothing else here changes.

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser};

use crate::environment::Environment;
use crate::failure::{Failure, exit};
use crate::invocation::{Globals, Invocation};
use crate::output::Output;
use crate::{api, auth, contexts, devices, sessions};

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
    /// Remote sessions (list, end).
    Session {
        #[command(subcommand)]
        command: sessions::SessionCommand,
    },
    /// A terminal on a device through the hub (login token only).
    Ssh(sessions::terminal::SshArgs),
    /// Files on a device through the hub (ls, get, put; login token only).
    Files {
        #[command(subcommand)]
        command: sessions::files::FilesCommand,
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
            Self::Session { command } => command.run(invocation).await,
            Self::Ssh(args) => args.run(invocation).await,
            Self::Files { command } => command.run(invocation).await,
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
