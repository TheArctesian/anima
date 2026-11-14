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
#[cfg(test)]
mod tests;
pub mod trail;
pub mod words;

use source_exchange::Registration;

use super::{acquisition, intake::Registry};

/// The source the flow connects.
pub const GITHUB: &str = "github";

/// How GitHub makes an App for the operator, from the hub's registry; `None` when the hub does
/// not know GitHub or GitHub could not make one.
#[must_use]
pub fn github(registry: &Registry) -> Option<Registration> {
    registry
        .declaration(GITHUB)
        .and_then(acquisition::registration)
}
