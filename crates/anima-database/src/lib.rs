//! The PostgreSQL that the desktop application carries (one-click plan S3, ruling OR-6).
//!
//! The hub reads a database URL from the file `ANIMA_DATABASE_URL_FILE` names (DB-002) and
//! cannot tell this database from the one in Docker. This crate owns everything on the other
//! side of that file: where the files are, the server process, the roles.

mod layout;

pub use layout::{Layout, LayoutError, Platform, Role, UserFolders};
