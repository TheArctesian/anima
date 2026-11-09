//! Session shares (RA-035, D-7, D-9, K4 §10): the operator who started a session shares it
//! with a link while its tunnel is open, and ends the share whenever they like.
//!
//! Shares live in memory only (S-1): a hub restart ends every session, and so every share.
//! Pure rules: the bounds of a share (`terms`) and its types (`model`). In memory: the registry
//! `Live` holds (`registry`). I/O: `shareSession` (`opening`), the task that ends each share
//! once (`keeper`), and the audit entries and events (`trail`). The GraphQL part is `graphql`.
//!
//! A share token is shown once, in `ShareLink.url` after `#`; the hub keeps only its SHA-256.
//! No log line, error, event or audit entry holds it (LOG-007).

mod graphql;
mod keeper;
mod model;
mod opening;
mod registry;
mod terms;
#[cfg(test)]
mod tests;
mod trail;

use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    clock::Clock,
    events::{self, Bus},
    ids::{SessionId, ShareId},
    sessions::Live,
};

pub use graphql::Mutation as ShareMutation;
pub use model::SessionShare;
pub(super) use registry::Shares;

/// What sharing needs from the hub: the database, the bus and the clock the audit entries and
/// events are written with, and the live state the shares and tunnels are held in. Cheap to
/// clone; each share's keeper holds a clone.
#[derive(Clone)]
pub(super) struct Sharing {
    pub(super) db: PgPool,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) live: Live,
}

/// Why a share operation failed. No variant holds a share token (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("session {0} not found")]
    UnknownSession(SessionId),
    /// No such share, or it has ended.
    #[error("share {0} not found")]
    UnknownShare(ShareId),
    /// The session, or the share, belongs to another operator.
    #[error("the session belongs to another operator")]
    NotOwner,
    /// The session has ended, or its tunnel is not open yet.
    #[error("the session is not open")]
    NotOpen,
    /// The hub holds the most shares it allows, or it is shutting down.
    #[error("no share can be created now")]
    Unavailable,
    /// The messages stay generic: the source errors may hold SQL text (API-010).
    #[error("session store error")]
    Sessions(#[from] crate::sessions::Error),
    #[error("event store error")]
    Events(#[from] events::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("no random bytes for a share token")]
    Random(#[source] getrandom::Error),
    /// The task that writes `share.create` and starts the keeper did not finish.
    #[error("the opening of a share stopped")]
    Stopped(#[source] tokio::task::JoinError),
}
