//! The PostgreSQL that the desktop application carries (one-click plan S3, ruling OR-6).
//!
//! The hub reads a database URL from the file `ANIMA_DATABASE_URL_FILE` names (DB-002) and
//! cannot tell this database from the one in Docker. This crate owns everything on the other
//! side of that file: where the files are, the server process, the roles.

pub mod backup;
mod cluster;
mod layout;
// Also for the program that runs the hub's group (S4): its key, state and logs are in the hub root.
pub mod owner_only;
mod password;
mod roles;
mod server;
pub mod startup;
mod tree;
mod urls;

#[cfg(test)]
mod scratch;

pub use cluster::ClusterError;
pub use layout::{Layout, LayoutError, Platform, Role, UserFolders};
pub use roles::RolesError;
pub use server::{Limits, ServerError, Settings, Stopped};
pub use tree::{Tree, TreeError};
pub use urls::UrlError;

use password::Password;
use server::{Lock, Server, lock, stop_left_over};
use tree::Program;
