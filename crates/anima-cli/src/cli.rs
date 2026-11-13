//! The command tree. Nouns are grouped by what the operator works on: `fleet` (devices and how
//! they are reached), `hub_records` (what the hub keeps) and `program` (the program itself). A new
//! noun adds one variant and one arm to the group it belongs to, beside its `mod` line in `lib.rs`.

use std::ffi::OsString;
use std::process::ExitCode;

use clap::Parser;

use crate::environment::Environment;
use crate::failure::{Failure, Hint, exit};
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
    let failure = usage_failure(&error.render().to_string());
    Output::new(true, environment.terminals.stdout, true).failure(&failure);
    ExitCode::from(exit::USAGE)
}

/// The CMD-022 failure for what clap printed: its first paragraph as the message, so the
/// missing flags are named, and the help of the command that was being run as the hint.
fn usage_failure(rendered: &str) -> Failure {
    let mut paragraph = rendered
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim);
    let first = paragraph.next().unwrap_or_default();
    // "the following required arguments were not provided:" lists them one to a line.
    let listed: Vec<&str> = paragraph.collect();
    let message = if listed.is_empty() {
        first.to_owned()
    } else {
        format!("{first} {}", listed.join(", "))
    };
    let command: Vec<&str> = rendered
        .lines()
        .find_map(|line| line.strip_prefix("Usage: "))
        .map_or_default(|usage| {
            usage
                .split_whitespace()
                .take_while(|word| !word.starts_with(['-', '<', '[']))
                .collect()
        });
    let command = if command.is_empty() {
        "anima".to_owned()
    } else {
        command.join(" ")
    };
    Failure::Usage {
        message: message.trim_start_matches("error: ").to_owned(),
        hint: Some(Hint::run(format!("{command} --help"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(args: &[&str]) -> String {
        Cli::try_parse_from(args)
            .expect_err("a usage error")
            .render()
            .to_string()
    }

    #[test]
    fn cmd_022_c1_a_missing_flag_is_named_and_the_hint_is_the_commands_help() {
        let failure = usage_failure(&rendered(&["anima", "key", "create"]));
        let Failure::Usage { message, hint } = failure else {
            panic!("a usage failure");
        };
        assert!(
            message.contains("--name <NAME>") && message.contains("--expires <EXPIRES>"),
            "{message}"
        );
        assert_eq!(hint, Some(Hint::run("anima key create --help")));
    }

    #[test]
    fn cmd_022_c1_an_unknown_flag_keeps_its_one_line() {
        let failure = usage_failure(&rendered(&["anima", "device", "list", "--nope"]));
        let Failure::Usage { message, hint } = failure else {
            panic!("a usage failure");
        };
        assert_eq!(message, "unexpected argument '--nope' found");
        assert_eq!(hint, Some(Hint::run("anima device list --help")));
    }
}
