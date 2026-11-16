//! Whether the hub can read a project repository on GitHub now, from the operator's choices and
//! the standing of the account that holds it (PRJ-032). Pure.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

use crate::{
    engine::sources::Standing,
    ids::{RepositoryId, SourceAccountId},
};

/// Whether the hub can read and change a repository on GitHub now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum RepositoryAccess {
    Ready,
    /// No source account holds it (the account was removed and the records kept).
    NotConnected,
    /// GitHub refused the key of its account.
    NeedsReconnect,
    /// Its account is paused.
    Paused,
    LeftOut,
    /// The connection no longer lists it.
    Gone,
}

/// The access of a repository: no account first (nothing else can be read), then the
/// operator's leave-out, then the listing, then the account's own standing. `standing` is
/// `None` when no account holds the repository.
#[must_use]
pub fn access(left_out: bool, gone: bool, standing: Option<Standing>) -> RepositoryAccess {
    match standing {
        None | Some(Standing::NotFound) => RepositoryAccess::NotConnected,
        Some(_) if left_out => RepositoryAccess::LeftOut,
        Some(_) if gone => RepositoryAccess::Gone,
        Some(Standing::Paused) => RepositoryAccess::Paused,
        Some(Standing::NeedsReconnect) => RepositoryAccess::NeedsReconnect,
        Some(Standing::Ready) => RepositoryAccess::Ready,
    }
}

/// The fixed words of a refused live read when the access is not READY (`CONFLICT`).
#[must_use]
pub const fn refusal(access: RepositoryAccess) -> &'static str {
    match access {
        RepositoryAccess::Ready => "The repository can be read.",
        RepositoryAccess::NotConnected => {
            "This repository is not connected: no source account holds it."
        }
        RepositoryAccess::NeedsReconnect => {
            "GitHub refused the key of its account. Connect the account again."
        }
        RepositoryAccess::Paused => "Its account is paused. Resume it to read the repository.",
        RepositoryAccess::LeftOut => "This repository is left out. Restore it to read it.",
        RepositoryAccess::Gone => "The GitHub connection no longer lists this repository.",
    }
}

/// A READY repository: the account that reads it and its `owner/name`, which the provider
/// checks again before it builds a path; its page on GitHub and its default branch as the
/// last sync read them.
#[derive(Clone, Debug)]
pub struct Target {
    pub repository: RepositoryId,
    pub account: SourceAccountId,
    pub name: String,
    /// The repository's page on GitHub, from GitHub (PRJ-010).
    pub home: String,
    /// `None` for an empty repository.
    pub default_branch: Option<String>,
}
