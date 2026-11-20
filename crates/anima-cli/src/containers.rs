//! `anima containers`: the containers of a device, read through the hub's SSH link (HOST-001,
//! HOST-013, HOST-017 to HOST-021, CMD-030). The lists are open to any token; a log and the
//! details of a container need a login (D-23). `check-file` and `from-run` (HOST-022) read
//! the text of a Compose file or of a `docker run` command, and reach no device.

pub mod check_file;
pub mod documents;
pub mod from_run;
pub mod inspect;
pub mod list;
pub mod logs;
pub mod networks;
pub mod projects;
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
    /// List the Compose projects of a device: state, running count, and where Compose found
    /// the project (its folder and files on the device).
    Projects(projects::ProjectsArgs),
    /// Show for each container whether its public registry has a newer image for its tag.
    /// Anima updates nothing.
    Updates(updates::UpdatesArgs),
    /// Check the syntax of a Compose file (`-` for standard input). No device is reached.
    /// It is a check of the syntax only: it does not say that a machine accepts the file.
    /// The exit status is 1 when the file has an error.
    CheckFile(check_file::CheckFileArgs),
    /// Change one `docker run` command into a Compose file: give the command after `--`,
    /// or on standard input. Nothing is run and no device is reached. The file goes to
    /// standard output and the notes to standard error.
    FromRun(from_run::FromRunArgs),
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
            Self::Projects(args) => args.run(invocation).await,
            Self::Updates(args) => args.run(invocation).await,
            Self::CheckFile(args) => args.run(invocation).await,
            Self::FromRun(args) => args.run(invocation).await,
        }
    }
}
