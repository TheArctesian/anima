//! The titles of records that other domains own (K-E4 Section 4): each domain that has records
//! to link gives a [`Titles`] at wiring, so `links` never imports it.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use sqlx::PgPool;

use crate::engine::records::Reference;

/// A domain could not give its titles now. The message holds no detail.
#[derive(Debug, thiserror::Error)]
#[error("the titles of {0} records are not available")]
pub struct Unavailable(pub &'static str);

/// The domain that owns the records of one kind, as far as their titles go.
#[async_trait]
pub trait Titles: Send + Sync {
    /// The record kind this domain owns, for example `place`.
    fn kind(&self) -> &'static str;

    /// The title of each id that exists. An id with no record is not in the answer.
    ///
    /// # Errors
    /// [`Unavailable`] when the domain cannot read its records.
    async fn titles(
        &self,
        db: &PgPool,
        ids: &[String],
    ) -> Result<HashMap<String, String>, Unavailable>;
}

/// Every [`Titles`] of the hub, built once in wiring.
#[derive(Clone, Default)]
pub struct TitleSources(Vec<Arc<dyn Titles>>);

impl TitleSources {
    #[must_use]
    pub fn new(sources: Vec<Arc<dyn Titles>>) -> Self {
        Self(sources)
    }

    /// The title of each of `records` that its domain knows. A record whose kind has no
    /// domain here, or whose record does not exist, is not in the answer.
    ///
    /// # Errors
    /// [`Unavailable`] from the first domain that cannot answer.
    pub async fn of(
        &self,
        db: &PgPool,
        records: &[&Reference],
    ) -> Result<HashMap<Reference, String>, Unavailable> {
        let mut found = HashMap::new();
        for source in &self.0 {
            let ids: Vec<String> = records
                .iter()
                .filter(|record| record.kind() == source.kind())
                .map(|record| record.id().to_owned())
                .collect();
            if ids.is_empty() {
                continue;
            }
            for (id, title) in source.titles(db, &ids).await? {
                if let Ok(record) = Reference::new(source.kind(), &id) {
                    found.insert(record, title);
                }
            }
        }
        Ok(found)
    }
}
