//! What a release carries for a hub on this computer (INS-002, S12 packaging): the programs, the
//! PostgreSQL tree, the bundle settings that name them, and the checks of the signed app.

mod mach_o;
mod programs;
mod signed_files;
mod tree;

use crate::builder::plan::Command;

fn args(command: &Command) -> Vec<&str> {
    command.args.iter().map(|a| a.to_str().unwrap()).collect()
}
