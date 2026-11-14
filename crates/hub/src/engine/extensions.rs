//! Which extensions the operator has switched on (EXT-001, EXT-011, EXT-012). The registry
//! lists the fifteen; the switches say which are on. Other domains ask [`on_keys`] or
//! [`require_on`]; they never read the table.

pub mod graphql;
mod model;
mod registry;
mod store;
mod switching;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use async_graphql::{Error as GraphQlError, ErrorExtensions, Value};
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use crate::api_errors::{ErrorCode, coded, internal};

pub use model::{Entry, Key, Navigation, NavigationGroup, Sensitivity};
pub use registry::{PROJECTS, registry};
pub use switching::Refusal;
use switching::plan;

/// Why an extensions operation failed. Messages hold no SQL and no input value.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// The extension a request needs is off.
    #[error("{} is off", .0.name())]
    Off(Key),
    #[error(transparent)]
    Refused(#[from] Refusal),
}

/// `CONFLICT` for an extension that is off or a refused switch (K7); anything the operator
/// cannot act on is `INTERNAL` with no detail (API-010).
impl Error {
    #[must_use]
    pub fn into_graphql(self) -> GraphQlError {
        let error = self;
        match error {
            Error::Database(_) => internal(error),
            Error::Off(key) => coded(
                format!("{} is off. Set it on in Settings.", key.name()),
                ErrorCode::Conflict,
            ),
            Error::Refused(Refusal::Needed { ref by, .. }) => {
                let keys = by.iter().map(|key| Value::from(key.as_str())).collect();
                coded(error.to_string(), ErrorCode::Conflict)
                    .extend_with(|_, extensions| extensions.set("neededBy", Value::List(keys)))
            }
            Error::Refused(refusal) => coded(refusal.to_string(), ErrorCode::Conflict),
        }
    }
}

/// The keys that are on: the stored switches that name an entry of `registry`, and every
/// fixed entry.
fn on_set<'a>(registry: &[Entry], stored: impl IntoIterator<Item = &'a str>) -> HashSet<Key> {
    let stored: HashSet<&str> = stored.into_iter().collect();
    registry
        .iter()
        .filter(|entry| entry.fixed || stored.contains(entry.key.as_str()))
        .map(|entry| entry.key)
        .collect()
}

/// Sets `key` to `want` in the caller's transaction and returns the keys whose switch changed,
/// the named one first. The table is locked while the plan is made, so two requests cannot both
/// plan from the same switches.
///
/// # Errors
/// `Refused` when [`plan`] refuses; `Database` when a statement fails. The caller rolls back.
async fn switch(
    conn: &mut PgConnection,
    registry: &[Entry],
    (key, want): (Key, bool),
    (at, by): (DateTime<Utc>, &str),
) -> Result<Vec<Key>, Error> {
    store::lock(conn).await?;
    let stored = store::switched_on(&mut *conn).await?;
    let on_now = on_set(registry, stored.iter().map(String::as_str));
    let changed = plan(registry, &on_now, key, want)?;
    for changed_key in &changed {
        store::set(conn, (*changed_key, want), (at, by)).await?;
    }
    Ok(changed)
}

/// The keys that are on right now; the scheduler and each run in flight read it (EXT-002).
///
/// # Errors
/// `Database` when the read fails.
pub async fn on_keys(db: &PgPool) -> Result<HashSet<Key>, Error> {
    let stored = store::switched_on(db).await?;
    Ok(on_set(registry(), stored.iter().map(String::as_str)))
}

/// Ok when `key` is on.
///
/// # Errors
/// `Off` (`CONFLICT` at the API) when it is off; `Database` when the read fails.
pub async fn require_on(db: &PgPool, key: Key) -> Result<(), Error> {
    if on_keys(db).await?.contains(&key) {
        Ok(())
    } else {
        Err(Error::Off(key))
    }
}
