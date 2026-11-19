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
mod live;
mod model;
mod registration;
mod run;
mod schedule;
mod scheduler;
mod store;
#[cfg(test)]
mod tests;
mod trail;

use std::{sync::Arc, time::Duration};

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
pub use connect::{Connection, Established, establish};
pub use feed::Feed;
pub use health::{HealthFacts, health};
pub use intake::{Counts, Intake, IntakeError, Page, Registry, Taken};
pub use live::{Keep, LiveFailure, Reply, Reserve, Standing, ask, change, forget, standing};
pub use model::{Account, ErrorKind, Health, Outcome, PauseReason, Trigger};
pub use registration::{
    attempts::Attempts,
    routes::{GithubFlow, router as github_connect_router},
};
pub use run::run;
pub use schedule::next;
pub use scheduler::{spawn_scheduler, start_due};

/// The sources' parts that `main.rs` builds once and the schema and the scheduler share: the
/// registry, the transport (with the in-memory tokens), the feed of changes and the one attempt
/// to connect GitHub. Clones share them.
#[derive(Clone)]
pub struct Reach {
    pub registry: Registry,
    pub transport: Transport,
    pub feed: Feed,
    pub attempts: Attempts,
}

/// How often the hub asks GitHub for the installation of an App that waits for Install (K-E1
/// Section 10a).
pub const INSTALLATION_POLL: Duration = Duration::from_secs(5);

/// How long the bound browser's page on `/connect/github/installed` waits for the attempt to end
/// before it shows the "finishing" page (K-E1 Section 10a).
pub const INSTALLED_PAGE_WAIT: Duration = Duration::from_secs(20);

impl Reach {
    #[must_use]
    pub fn new(registry: Registry, transport: Transport) -> Self {
        Self {
            registry,
            transport,
            feed: Feed::new(),
            attempts: Attempts::new(INSTALLATION_POLL, INSTALLED_PAGE_WAIT),
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

/// The health of each account at `now`, with its ID and its label, oldest account first: for
/// a reader that follows the changes of health (K-E6 Section 2, `source_state`).
///
/// # Errors
/// `Database`, `Extensions`.
pub async fn healths(
    db: &PgPool,
    registry: &Registry,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<(SourceAccountId, String, Health)>, Error> {
    let on = extensions::on_keys(db).await?;
    let accounts = store::accounts(db, None).await?;
    Ok(accounts
        .into_iter()
        .map(|account| {
            let declared = registry.declaration(&account.source);
            let facts = HealthFacts {
                paused: account.paused_at.is_some(),
                consumer_on: registry.consumer_on(&account.source, &on),
                last_error_kind: account.error_kind(),
                last_started_at: account.last_started_at,
                last_sync_at: account.last_sync_at,
                failure_count: account.failures(),
                interval: schedule::interval(declared.map_or(0, |d| d.poll_interval_seconds)),
            };
            (account.id, account.label, health(&facts, now))
        })
        .collect())
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
