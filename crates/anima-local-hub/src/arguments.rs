//! The words of the command line: the commands of `anima-local-hub` and their options. `main.rs`
//! wires each command to the library.

use std::net::SocketAddr;
use std::path::PathBuf;

use anima_database::{LayoutError, Platform, UserFolders};
use anima_local_hub::account::Name;
use anima_local_hub::{Carried, Folders, Pool};
use clap::{Args, Parser, Subcommand};

use crate::LISTEN;

#[derive(Parser)]
#[command(name = "anima-local-hub", about = "The Anima hub on this computer")]
pub(crate) struct Arguments {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Start the hub's programs in order and keep them up until SIGTERM
    Run {
        #[command(flatten)]
        place: Place,
        /// Where the hub listens; a loopback address
        #[arg(long, default_value = LISTEN)]
        listen: SocketAddr,
        #[command(flatten)]
        programs: RunFiles,
    },
    /// Copy the programs out of the application and start the hub at each login
    Install {
        #[command(flatten)]
        place: Place,
        /// The PostgreSQL tree that the application carries
        #[arg(long)]
        tree: PathBuf,
        /// Where the hub listens, when it is not the default
        #[arg(long)]
        listen: Option<SocketAddr>,
        /// Put the programs of this application over installed ones of a higher version
        #[arg(long)]
        step_back: bool,
    },
    /// Replace the programs of the installed hub when the application carries other ones
    Update {
        #[command(flatten)]
        place: Place,
        /// The PostgreSQL tree that the application carries
        #[arg(long)]
        tree: PathBuf,
    },
    /// Stop the hub and remove its unit and programs; the database and the key stay
    Remove(Place),
    /// Print as JSON whether the hub is installed and runs, and its last state
    Status(Place),
    /// Start the installed hub
    Start(Place),
    /// Stop the hub until `start` or the next login; it stays installed
    Stop(Place),
    /// Make a copy of the whole folder of the stopped hub, with a dump of its database
    Copy {
        #[command(flatten)]
        place: Place,
        /// The PostgreSQL tree: a folder with bin/, lib/ and share/
        #[arg(long)]
        tree: PathBuf,
        /// The folder of the copy; it must not exist
        #[arg(long)]
        to: PathBuf,
    },
    /// Check a copy that `copy` made: each file and the dump; it changes nothing
    CheckCopy {
        /// The folder of the copy
        copy: PathBuf,
        /// The PostgreSQL tree: a folder with bin/, lib/ and share/
        #[arg(long)]
        tree: PathBuf,
    },
    /// The operator account, through the installed hub; secrets pass on standard input
    Account {
        #[command(flatten)]
        place: Place,
        #[arg(value_enum)]
        verb: AccountVerb,
        /// The operator's name: for `totp-new` and `create`
        #[arg(value_parser = Name::parse)]
        name: Option<Name>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub(crate) enum AccountVerb {
    /// One JSON line: the operator's name and whether TOTP is set, or null
    Status,
    /// One JSON line with a new TOTP secret and its URI; nothing is saved
    TotpNew,
    /// Make or replace the operator: password, secret and code on three lines of input
    Create,
}

#[derive(Args)]
pub(crate) struct Place {
    /// The hub's folder; the default is the application's folder of this user
    #[arg(long, requires = "socket_folder")]
    pub(crate) root: Option<PathBuf>,
    /// The folder for the database's Unix socket
    #[arg(long, requires = "root")]
    pub(crate) socket_folder: Option<PathBuf>,
}

impl Place {
    pub(crate) fn folders(&self) -> Result<Folders, LayoutError> {
        match (&self.root, &self.socket_folder) {
            (Some(root), Some(socket_folder)) => Folders::new(root.clone(), socket_folder.clone()),
            _ => Folders::for_user(
                Platform::HOST,
                &UserFolders {
                    home: std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                        .map_or_else(PathBuf::new, PathBuf::from),
                    #[cfg(windows)]
                    local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
                    xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
                    xdg_runtime_dir: std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from),
                },
            ),
        }
    }
}

/// Where the programs are, when they are not around this program as `install` puts them.
#[derive(Args)]
pub(crate) struct RunFiles {
    /// The folder with anima-hub, anima-database and anima-gateway
    #[arg(long, requires = "tree")]
    programs: Option<PathBuf>,
    /// The PostgreSQL tree: a folder with bin/, lib/ and share/
    #[arg(long, requires = "programs")]
    tree: Option<PathBuf>,
    /// The file of the Anima.ing cloud's key, on a hub of the pool only: the hub runs in the
    /// hosted mode (R-129). The hub refuses a path that is not absolute
    #[arg(long)]
    pub(crate) hosted_cloud_key_file: Option<PathBuf>,
    /// The address at which a browser reaches this hub through the gateway, on a hub of the
    /// pool only: `https://` and the host of the gateway. The hub refuses an address that is
    /// not an origin
    #[arg(long)]
    pub(crate) public_origin: Option<String>,
    /// The folder of the web client that the hub serves, on a hub of the pool only
    #[arg(long)]
    pub(crate) web_root: Option<PathBuf>,
}

impl RunFiles {
    /// What only a hub of the pool is told.
    pub(crate) fn pool(&self) -> Pool {
        Pool {
            cloud_key_file: self.hosted_cloud_key_file.clone(),
            public_origin: self.public_origin.clone(),
            web_root: self.web_root.clone(),
        }
    }

    pub(crate) fn carried(self) -> Result<Carried, crate::Failure> {
        if let (Some(programs), Some(tree)) = (self.programs, self.tree) {
            return Ok(Carried::new(programs, tree));
        }
        Carried::around(&std::env::current_exe()?)
            .ok_or_else(|| "this program is not in a bin folder beside its PostgreSQL tree".into())
    }
}
