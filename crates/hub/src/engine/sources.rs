//! The places the hub reads records from, and its accounts with them (SRC-001 to SRC-012,
//! SRC-021, PRJ-018, K-E1).
//!
//! An account holds a reference to its sealed App key; the installation token it mints lives
//! in memory only. The scheduler runs each due account whose consumer extension is on (EXT-002)
//! and hands each page to the consumer's [`Intake`]; the cursor moves only in the transaction
//! that stored the page (SRC-007). There is one hub: the database row's generation fences a
//! run, and a second server with failover is deferred (GitHub issue 5).

pub mod acquisition;
mod connect;
mod controls;
mod feed;
pub mod graphql;
mod health;
mod intake;
mod model;
mod run;
mod schedule;
mod scheduler;
mod store;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use async_graphql::Error as GraphQlError;
use sqlx::{PgConnection, PgPool};

use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid},
    clock::Clock,
    engine::extensions,
    fleet::{credentials, events::Bus},
    ids::SourceAccountId,
};

pub use acquisition::{
    app_key::KeyRefusal,
    tokens::Tokens,
    transport::{BuildFailed, Transport},
};
pub use feed::Feed;
pub use health::{HealthFacts, health};
pub use intake::{Counts, Intake, IntakeError, Page, Registry, Taken};
pub use model::{Account, ErrorKind, Health, Outcome, PauseReason, Trigger};
pub use run::run;
pub use schedule::next;
pub use scheduler::{spawn_scheduler, start_due};

/// The sources' parts that `main.rs` builds once and the schema and the scheduler share: the
/// registry, the transport (with the in-memory tokens) and the feed of changes. Clones share
/// them.
#[derive(Clone)]
pub struct Reach {
    pub registry: Registry,
    pub transport: Transport,
    pub feed: Feed,
}

impl Reach {
    #[must_use]
    pub fn new(registry: Registry, transport: Transport) -> Self {
        Self {
            registry,
            transport,
            feed: Feed::new(),
        }
    }
}

/// What a run needs (K3).
#[derive(Clone)]
pub struct Scheduling {
    pub pool: PgPool,
    pub bus: Bus,
    pub clock: Arc<dyn Clock>,
    pub vault: credentials::Vault,
    pub registry: Registry,
    pub transport: Transport,
    pub feed: Feed,
}

impl Scheduling {
    /// The parts of the hub a run needs, with the sources' `reach`.
    #[must_use]
    pub fn new(
        (pool, bus, clock): (PgPool, Bus, Arc<dyn Clock>),
        vault: credentials::Vault,
        reach: Reach,
    ) -> Self {
        Self {
            pool,
            bus,
            clock,
            vault,
            registry: reach.registry,
            transport: reach.transport,
            feed: reach.feed,
        }
    }
}

/// Why a sources operation failed. Fixed words: never a key, a token or a provider reply.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("the credential store failed")]
    Credentials(#[from] credentials::Error),
    #[error("the extension switches could not be read")]
    Extensions(#[from] extensions::Error),
    #[error("the records could not be deleted")]
    Intake(#[from] IntakeError),
    #[error("no source account has this ID")]
    NotFound,
    #[error("the account is paused")]
    Paused,
    #[error("no extension that uses this source is on")]
    NoConsumer,
    #[error("the hub does not know this source")]
    UnknownSource,
    #[error("the label must be 1 to 64 characters")]
    Label,
    #[error("{0}")]
    Key(#[from] KeyRefusal),
    #[error("the source refused the key")]
    KeyRefused,
    #[error("the source could not be reached")]
    Unreachable,
}

impl Error {
    /// `NOT_FOUND`, `CONFLICT`, `VALIDATION` or `UNAVAILABLE` for what the operator can act
    /// on; `INTERNAL` with no detail otherwise (K7, API-010).
    #[must_use]
    pub fn into_graphql(self) -> GraphQlError {
        match self {
            Self::NotFound => coded("No source account has this ID.", ErrorCode::NotFound),
            Self::Paused => coded(
                "The account is paused. Resume it first.",
                ErrorCode::Conflict,
            ),
            Self::NoConsumer => coded(
                "No extension that uses this source is on. Set one on in Settings.",
                ErrorCode::Conflict,
            ),
            Self::Label => invalid(&[FieldError::new(
                "label",
                "Give the account a label of 1 to 64 characters.",
            )]),
            Self::Key(refusal) => invalid(&[FieldError::new(refusal.field(), refusal.to_string())]),
            Self::KeyRefused => invalid(&[FieldError::new(
                "privateKey",
                "The source refused this key for this App and installation.",
            )]),
            Self::Unreachable => coded(
                "The source cannot be reached. Try again later.",
                ErrorCode::Unavailable,
            ),
            other => internal(other),
        }
    }
}

/// Makes the account due now, in the caller's transaction (used by an extension that needs
/// fresh records, such as `projects` after the operator restores a repository).
///
/// # Errors
/// `NotFound` when there is no such account; `Database` when the write fails.
pub async fn request_sync(conn: &mut PgConnection, account: SourceAccountId) -> Result<(), Error> {
    if store::request_sync(conn, account).await? {
        Ok(())
    } else {
        Err(Error::NotFound)
    }
}
