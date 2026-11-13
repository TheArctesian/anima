//! The command tree. Nouns are grouped by what the operator works on: `fleet` (devices and how
//! they are reached), `hub_records` (what the hub keeps) and `program` (the program itself). A new
//! noun adds one variant and one arm to the group it belongs to, beside its `mod` line in `lib.rs`.

use std::ffi::OsString;
use std::process::ExitCode;

use clap::Parser;

use crate::environment::Environment;
use crate::failure::{Failure, exit};
use crate::invocation::{Globals, Invocation};
use crate::output::Output;

mod fleet;
mod hub_records;
mod program;

use fleet::FleetCommand;
use hub_records::HubRecordCommand;
use program::ProgramCommand;

/// The command line for an Anima hub.
#[derive(Debug, Parser)]
#[command(name = "anima", version, about, propagate_version = true)]
pub struct Cli {
    #[command(flatten)]
    globals: Globals,
    #[command(subcommand)]
    command: Command,
}

/// The groups the operator thinks in; each noun lives in exactly one.
#[derive(Debug, clap::Subcommand)]
enum Command {
    #[command(flatten)]
    Fleet(FleetCommand),
    #[command(flatten)]
    HubRecord(HubRecordCommand),
    #[command(flatten)]
    Program(ProgramCommand),
}

impl Command {
    async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Fleet(command) => command.run(invocation).await,
            Self::HubRecord(command) => command.run(invocation).await,
            Self::Program(command) => command.run(invocation).await,
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
