//! The hub on this computer (one-click plan S4, S6).
//!
//! The operating system keeps one program up, `anima-local-hub run`; that program keeps the
//! hub's group up in order: the database (S3), the gateway (S2), the hub, and the link (S6),
//! which alone is started again when it ends. Nothing here knows what the hub does, and the
//! hub does not know that this program started it.

pub mod account;
mod child;
mod folders;
pub mod group;
mod link_member;
mod log_file;
mod members;
mod plain_file;
pub mod programs;
pub mod run;
pub mod state;
mod stop_marker;
pub mod units;
mod vault_key;

pub use child::{ChildError, Stop};
pub use folders::Folders;
pub use link_member::Backoff;
pub use members::Carried;
pub use vault_key::KeyError;
