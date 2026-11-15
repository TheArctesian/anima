//! The PostgreSQL that the desktop application carries (one-click plan S3, ruling OR-6).
//!
//! The hub reads a database URL from the file `ANIMA_DATABASE_URL_FILE` names (DB-002) and
//! cannot tell this database from the one in Docker. This crate owns everything on the other
//! side of that file: where the files are, the server process, the roles.

pub mod cluster;
mod layout;
mod owner_only;
mod password;
pub mod roles;
mod server;
mod tree;
pub mod urls;

#[cfg(test)]
mod local_tree;
#[cfg(test)]
mod scratch;

pub use layout::{Layout, LayoutError, Platform, Role, UserFolders};
pub use password::Password;
pub use server::{Limits, Lock, Server, ServerError, Settings, Stopped, lock, stop_left_over};
pub use tree::{Program, Tree, TreeError};
