//! API keys for scripts: named, READ or WRITE, with an expiry at most one year away. The secret
//! is shown once; the hub keeps only its SHA-256 and the first 12 characters. Each change has
//! its audit entry and event (INT-003, LOG-005, D-23, P3-004).

mod graphql;
mod rules;
#[cfg(test)]
mod tests;
mod trail;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use zeroize::Zeroizing;

pub(super) use graphql::{ApiKeyCreated, ApiKeyInput, api_keys, create_api_key, revoke_api_key};

use super::{
    Scope,
    store::{self, KEY_NAME_INDEX, NewKey, StoredKey},
    tokens::TOKEN_BYTES,
};
use crate::{
    api_errors::FieldError,
    events::{self, Trail},
    ids::{ApiKeyId, OperatorId},
};

/// An API key as the operator sees it (K1 `ApiKey`); never its hash or its secret.
pub struct ApiKey {
    id: ApiKeyId,
    name: String,
    scope: Scope,
    /// The first 12 characters of the secret.
    prefix: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

/// A key just made and its secret, which is shown once. No `Debug`: it holds the secret.
struct Created {
    key: ApiKey,
    secret: Zeroizing<String>,
}

/// What the operator asks for (K1 `ApiKeyInput`).
struct Wanted {
    name: String,
    scope: Scope,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Some input is not valid.")]
    Invalid(Vec<FieldError>),
    #[error("Another API key has this name. Choose another.")]
    NameTaken,
    #[error("No such API key.")]
    NotFound,
    /// A stored row holds a scope this hub does not know, or one no API key has.
    #[error("an API key row cannot be read")]
    Unreadable,
    #[error("database error")]
    Database(#[source] sqlx::Error),
    #[error("audit or event write failed")]
    Trail(#[from] events::Error),
}

impl ApiKey {
    /// The key a stored row describes.
    fn from_row(row: StoredKey) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            scope: Scope::from_stored(&row.scope)
                .filter(|scope| *scope != Scope::Builder)
                .ok_or(Error::Unreadable)?,
            prefix: row.display_prefix,
            expires_at: row.expires_at,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
        })
    }
}

/// The operator's API keys, by name; expired ones too until they are pruned.
async fn list(db: impl PgExecutor<'_>, operator: OperatorId) -> Result<Vec<ApiKey>, Error> {
    let rows = store::keys_of(db, operator)
        .await
        .map_err(Error::Database)?;
    rows.into_iter().map(ApiKey::from_row).collect()
}

/// Checks `wanted` at the trail's time and stores it as a key of the trail's operator whose
/// secret is made of `bytes`, with its audit entry and event. The caller commits.
async fn create(
    conn: &mut PgConnection,
    trail: Trail<'_>,
    wanted: &Wanted,
    bytes: [u8; TOKEN_BYTES],
) -> Result<Created, Error> {
    let draft = rules::draft(wanted, bytes, trail.now).map_err(Error::Invalid)?;
    let new_key = NewKey {
        id: uuid::Uuid::now_v7().into(),
        operator: trail.operator(),
        hash: draft.token.hash(),
        name: &draft.name,
        scope: draft.scope,
        display_prefix: &draft.token.key_display_prefix(),
        expires_at: draft.expires_at,
        created_at: trail.now,
    };
    let key = ApiKey::from_row(
        store::insert_key(conn, &new_key)
            .await
            .map_err(write_error)?,
    )?;
    trail.write(conn, trail::created(&key)).await?;
    Ok(Created {
        key,
        secret: draft.token.key_secret(),
    })
}

/// Deletes the trail operator's key `id`, with its audit entry and event; the key stops at
/// once. The caller commits.
async fn revoke(conn: &mut PgConnection, trail: Trail<'_>, id: ApiKeyId) -> Result<(), Error> {
    let row = store::delete_key(conn, trail.operator(), id)
        .await
        .map_err(Error::Database)?
        .ok_or(Error::NotFound)?;
    trail.write(conn, trail::revoked(id, &row.name)).await?;
    Ok(())
}

/// A taken name is the operator's to fix; any other failure is the hub's.
fn write_error(error: sqlx::Error) -> Error {
    let constraint = error.as_database_error().and_then(|e| e.constraint());
    if constraint == Some(KEY_NAME_INDEX) {
        Error::NameTaken
    } else {
        Error::Database(error)
    }
}
