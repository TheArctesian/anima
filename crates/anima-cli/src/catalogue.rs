//! `anima catalogue`: the catalogue of applications that ships with Anima (HOST-047, task
//! H38, CMD-030). `list` and `show` read the entries that the hub lists; `install` asks for
//! the install of one entry on a device, which the hub refuses with a fixed reason until task
//! H6 exists. No command reaches a machine.

pub mod documents;
pub mod install;
pub mod list;
pub mod show;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum CatalogueCommand {
    /// List the entries of the catalogue: kind, version, marks, the day of the last test on a
    /// machine and the day of the last pin of the images.
    #[command(visible_alias = "ls")]
    List(list::ListArgs),
    /// Show one entry: its fields, the reason why the hub does not install it, the names of
    /// its environment variables and its Compose file.
    Show(show::ShowArgs),
    /// Ask the hub to install one entry on a device. The hub refuses today and says why; the
    /// exit status is 1.
    Install(install::InstallArgs),
}

impl CatalogueCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown entry or device, the hub's refusal, or a refused install.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Show(args) => args.run(invocation).await,
            Self::Install(args) => args.run(invocation).await,
        }
    }
}
