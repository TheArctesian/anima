//! Connect GitHub in three presses: the hub makes the operator's GitHub App from a manifest,
//! keeps its key and stores the connection when the App is installed (PRJ-039 to PRJ-042, K-E1
//! Section 7a).
//!
//! One attempt exists at a time, in hub memory only. A one-time ticket, a binding cookie and a
//! state sent to GitHub tie one browser to it; the hub keeps only their hashes. The private key
//! lives inside the attempt from "Create" to "Install" and goes to the one storing path
//! (`establish`) at the end.

pub mod attempt;
pub mod attempts;
pub mod binding;
mod exchange;
mod finish;
pub mod installation;
mod pages;
mod query;
pub mod routes;
#[cfg(test)]
mod tests;
pub mod trail;
pub mod words;

use source_exchange::Registration;

use super::{acquisition, intake::Registry};
use crate::config::Config;

/// The source the flow connects.
pub const GITHUB: &str = "github";

/// The origin at which a browser opens the pages of the flow, or `None` when no browser can
/// open this hub (HMD-070): a hub in the link mode that was given no public origin. Its pages
/// and the App that GitHub would make would name the default origin, which is a different
/// server. A hub of the pool is given the address of its gateway (HMD-069); a hub on a server
/// is reached at its own origin and is not in the link mode.
#[must_use]
pub fn browser_origin(config: &Config) -> Option<&str> {
    let unaddressed = config.link_socket.is_some() && !config.public_origin_given;
    (!unaddressed).then_some(config.public_origin.as_str())
}

/// How GitHub makes an App for the operator, from the hub's registry; `None` when the hub does
/// not know GitHub or GitHub could not make one.
#[must_use]
pub fn github(registry: &Registry) -> Option<Registration> {
    registry
        .declaration(GITHUB)
        .and_then(acquisition::registration)
}
