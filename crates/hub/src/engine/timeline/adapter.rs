//! What a domain hands the timeline so its records show there (K-E3; TML-001).
//!
//! The engine imports no extension (EXT-005): wiring builds the [`Adapters`] and the timeline
//! asks each one when it is read.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use super::model::{Ask, Evidence};

/// An adapter could not read its records. The message names the adapter only; the cause goes
/// to the process log (API-010).
#[derive(Debug, thiserror::Error)]
#[error("the timeline adapter {adapter} could not read its records")]
pub struct Unavailable {
    pub adapter: &'static str,
    #[source]
    pub cause: Box<dyn std::error::Error + Send + Sync>,
}

/// One domain's records as Evidence.
///
/// Obligations: copy nothing (TML-001); return only entries `ask` wants (`Ask::wants`), at
/// most `ask.limit`, newest first; never drop an entry because a second source recorded the
/// same event (TML-005); write an uncertainty note for each value the row does not give
/// (TML-006).
#[async_trait]
pub trait Adapter: Send + Sync {
    /// Its stable name, for the process log.
    fn name(&self) -> &'static str;

    /// The entries `ask` wants.
    ///
    /// # Errors
    /// [`Unavailable`] when the records cannot be read.
    async fn evidence(&self, db: &PgPool, ask: &Ask) -> Result<Vec<Evidence>, Unavailable>;
}

/// Every adapter of this hub, built once in wiring.
#[derive(Clone, Default)]
pub struct Adapters(Arc<[Arc<dyn Adapter>]>);

impl Adapters {
    #[must_use]
    pub fn new(adapters: Vec<Arc<dyn Adapter>>) -> Self {
        Self(adapters.into())
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &Arc<dyn Adapter>> {
        self.0.iter()
    }
}
