//! The catalogue that ships with Anima, in the hub (HOST-047, R-300,
//! R-299, task H38): the list of the entries and the description of one.
//!
//! The recipes are built into the hub binary (`catalogue::shipped`). The hub runs the static
//! test of each recipe once, at the first read, and lists only the recipes with no finding.
//! The install is present and always refused with a fixed reason: it needs task H6, which is
//! not built. No read and no install reaches a machine, keeps anything or writes a log line.
//! No domain imports this one; `schema.rs` merges [`graphql`].

pub mod graphql;
mod install;
mod listing;
mod model;
mod parts;
#[cfg(test)]
mod tests;

use std::sync::LazyLock;

pub use self::install::{Ask, Install, Refusal, install};
pub use self::model::{EntryKind, Listed, MarkKind};

/// The entries that passed the static test, in the order of their identifiers.
static LISTED: LazyLock<Vec<Listed>> = LazyLock::new(|| listing::listed(::catalogue::shipped()));

/// Each entry of the catalogue that the hub lists.
#[must_use]
pub fn entries() -> &'static [Listed] {
    &LISTED
}

/// The listed entry with the identifier `id`, if there is one.
#[must_use]
pub fn entry(id: &str) -> Option<&'static Listed> {
    entries().iter().find(|listed| listed.entry.id == id)
}
