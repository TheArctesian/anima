//! `anima containers`: the containers of a device, read through the hub's SSH link (HOST-001,
//! HOST-013, HOST-017 to HOST-021, CMD-030). The lists are open to any token; a log and the
//! details of a container need a login (D-23).

pub mod documents;
pub mod inspect;
pub mod list;
pub mod logs;
pub mod networks;
pub mod storage;
pub mod updates;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ContainerCommand {
    /// List the containers of a device: name, state, image, ports, Compose project.
    #[command(visible_alias = "ls")]
    List(list::ListArgs),
    /// Print the last lines of the log of one container (needs a login, not an API key).
    Logs(logs::LogsArgs),
    /// Show the details of one container: command, restart policy, health check, mounts,
    /// networks, labels, processes, and the names of its environment variables (needs a
    /// login, not an API key).
    Inspect(inspect::InspectArgs),
    /// List the images of a device, with the size of each and the space a clean-up can free.
    Images(storage::StorageArgs),
    /// List the volumes of a device, with the size of each and the space a clean-up can free.
    Volumes(storage::StorageArgs),
    /// List the networks of Docker on a device, with the containers on each.
    Networks(networks::NetworksArgs),
    /// Show for each container whether its public registry has a newer image for its tag.
    /// Anima updates nothing.
    Updates(updates::UpdatesArgs),
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
            Self::Inspect(args) => args.run(invocation).await,
            Self::Images(args) => args.images(invocation).await,
            Self::Volumes(args) => args.volumes(invocation).await,
            Self::Networks(args) => args.run(invocation).await,
            Self::Updates(args) => args.run(invocation).await,
        }
    }
}
