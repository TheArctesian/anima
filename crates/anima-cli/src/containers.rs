//! `anima containers`: the containers of a device, read through the hub's SSH link (HOST-001,
//! HOST-013, CMD-030). The list is open to any token; a log needs a login (D-23).

pub mod documents;
pub mod list;
pub mod logs;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ContainerCommand {
    /// List the containers of a device: name, state, image, ports, Compose project.
    #[command(visible_alias = "ls")]
    List(list::ListArgs),
    /// Print the last lines of the log of one container (needs a login, not an API key).
    Logs(logs::LogsArgs),
}

impl ContainerCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown device or container, a device the hub cannot read, or a hub
    /// refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Logs(args) => args.run(invocation).await,
        }
    }
}
