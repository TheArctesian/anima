//! `anima`: reads the process's arguments and environment once and runs the command.

use std::process::ExitCode;

use anima_cli::environment::Environment;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    anima_cli::cli::run(std::env::args_os().collect(), Environment::from_process()).await
}
