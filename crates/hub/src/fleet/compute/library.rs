//! The model library: models, their variants and files, aliases and the approved sources
//! (compute plan, LIBM-001 to LIBM-009, SEL-003).
//!
//! Reads take the pool. Writes take the caller's connection so the GraphQL layer adds its
//! audit entry in the same transaction (LOG-005). A repository is read through [`Fetch`], the
//! I/O edge, before the transaction starts. Messages in [`Error`] hold no input value
//! (LOG-007).

pub mod capability;
mod fetch;
pub mod graphql;
mod model;
mod naming;
mod reading;
mod repository;
mod store;
#[cfg(test)]
mod tests;
mod variants;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use crate::api_errors::{FieldError, field_names};

pub use capability::Capability;
pub use fetch::{Fetch, FetchError, HuggingFace};
pub use model::{Alias, Facts, Format, LayerShape, Missing, Model, Refusal, Variant, VariantFile};
pub use reading::Read;

/// Why a library operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the model is not in the library")]
    NotFound(Missing),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    #[error("the repository is refused: {0}")]
    Refused(Refusal),
    #[error("the source of the models failed: {0}")]
    Source(#[from] FetchError),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// A stored row is not in a form this version reads.
    #[error("a stored model cannot be read")]
    Unreadable,
}

fn invalid(error: FieldError) -> Error {
    Error::Validation(vec![error])
}

/// Every model in name order, with its variants and aliases (SEL-005).
///
/// # Errors
/// `Database` when a query fails.
pub async fn list(db: &PgPool) -> Result<Vec<Model>, Error> {
    let mut conn = db.acquire().await?;
    store::load(&mut conn, None).await
}

/// The model of this name or alias; a `name:tag` gives its model (LIBM-006, LIBM-007).
///
/// # Errors
/// `NotFound` when no model has the name or the alias; `Database` when a query fails.
pub async fn get(db: &PgPool, name: &str) -> Result<Model, Error> {
    let mut conn = db.acquire().await?;
    let (id, _) = store::resolve(&mut *conn, naming::model_part(name.trim()))
        .await?
        .ok_or(Error::NotFound(Missing::Model))?;
    store::load(&mut conn, Some(id))
        .await?
        .pop()
        .ok_or(Error::NotFound(Missing::Model))
}

/// The approved organisations in name order (LIBM-003).
///
/// # Errors
/// `Database` when the query fails.
pub async fn sources(db: &PgPool) -> Result<Vec<String>, Error> {
    store::sources(db).await
}

/// Adds an organisation to the approved list; returns false when it was there already.
///
/// # Errors
/// `Validation` (`organisation`) when the name is not an organisation name.
pub async fn add_source(
    conn: &mut PgConnection,
    organisation: &str,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let organisation = naming::organisation("organisation", organisation).map_err(invalid)?;
    store::add_source(conn, &organisation, now).await
}

/// Reads a repository for a model (LIBM-004). `name` is the model name; with none, the hub
/// makes one from the repository name. Nothing is stored: [`save`] stores the result.
///
/// # Errors
/// `Validation` (`repository`, `name`); `Refused` when the source is not approved or the
/// repository gives no variant; `Source` when the source fails.
pub async fn read(
    db: &PgPool,
    fetch: &impl Fetch,
    repository: &str,
    name: Option<&str>,
) -> Result<Read, Error> {
    let source = naming::repository(repository).map_err(invalid)?;
    let name = match name {
        Some(text) => naming::name("name", text).map_err(invalid)?,
        None => naming::name("name", &naming::proposed(&source)).map_err(invalid)?,
    };
    let approved = store::sources(db).await?;
    let existing = match get(db, &name).await {
        Ok(model) if model.name == name => Some(model),
        Ok(_) => {
            return Err(invalid(FieldError::new(
                "name",
                "The name is an alias. Use a different name.",
            )));
        }
        Err(Error::NotFound(_)) => None,
        Err(other) => return Err(other),
    };
    let known = reading::Known {
        approved: &approved,
        publisher: existing
            .as_ref()
            .and_then(|model| model.facts.publisher.as_deref()),
    };
    reading::read(fetch, &source, name, known).await
}

/// Stores what [`read`] gave: the model, and each variant in place of a variant of the same
/// tag. Returns the stored model.
///
/// # Errors
/// `Database` when a write fails.
pub async fn save(
    conn: &mut PgConnection,
    read: &Read,
    now: DateTime<Utc>,
) -> Result<Model, Error> {
    let id = store::save_model(conn, &read.name, &read.facts, now).await?;
    for variant in &read.variants {
        store::save_variant(conn, id, variant).await?;
    }
    store::load(conn, Some(id))
        .await?
        .pop()
        .ok_or(Error::NotFound(Missing::Model))
}

/// Points an alias at a model, in place of the model it pointed at (LIBM-007, SEL-003).
/// Returns the alias and the name of the model it pointed at before.
///
/// # Errors
/// `Validation` (`alias`) when the alias breaks the name rule or is the name of a model;
/// `NotFound` when there is no such model.
pub async fn set_alias(
    conn: &mut PgConnection,
    alias: &str,
    model: &str,
) -> Result<(Alias, Option<String>), Error> {
    let alias = naming::name("alias", alias).map_err(invalid)?;
    if let Some((_, name)) = store::resolve(&mut *conn, &alias).await?
        && name == alias
    {
        return Err(invalid(FieldError::new(
            "alias",
            "A model has this name. Use a different alias.",
        )));
    }
    let (id, name) = store::resolve(&mut *conn, naming::model_part(model.trim()))
        .await?
        .ok_or(Error::NotFound(Missing::Model))?;
    let before = store::set_alias(conn, &alias, id).await?;
    Ok((Alias { alias, model: name }, before))
}
