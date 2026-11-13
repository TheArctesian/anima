//! The SQL for API keys: the `api_key` rows of `access.api_tokens` (D-23, K2 0010-0029).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor};

use crate::{
    access::{
        MadeFor,
        tokens::{Scope, TokenHash, TokenKind},
    },
    ids::{ApiKeyId, OperatorId},
};

/// The partial unique index that keeps API key names apart (migration 0010).
pub(in crate::access) const KEY_NAME_INDEX: &str = "api_tokens_key_name";

/// The columns of an API key the operator may see; never the hash.
const KEY_COLUMNS: &str =
    "id, name, scope, made_for, display_prefix, expires_at, created_at, last_used_at";

/// An API key row to insert.
pub(in crate::access) struct NewKey<'a> {
    pub id: ApiKeyId,
    pub operator: OperatorId,
    pub hash: TokenHash,
    pub name: &'a str,
    pub scope: Scope,
    pub made_for: MadeFor,
    pub display_prefix: &'a str,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// An API key as stored, without its hash.
#[derive(FromRow)]
pub(in crate::access) struct StoredKey {
    pub id: ApiKeyId,
    pub name: String,
    pub scope: String,
    /// Null only for a row that is no API key; migration 0011 checks that.
    pub made_for: Option<String>,
    pub display_prefix: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Inserts the key and returns it as stored. A name another key has fails on
/// [`KEY_NAME_INDEX`].
pub(in crate::access) async fn insert_key(
    conn: &mut PgConnection,
    key: &NewKey<'_>,
) -> Result<StoredKey, sqlx::Error> {
    sqlx::query_as(&format!(
        "insert into access.api_tokens (id, operator_id, token_hash, kind, name, scope, \
         made_for, display_prefix, expires_at, created_at) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) returning {KEY_COLUMNS}"
    ))
    .bind(key.id)
    .bind(key.operator)
    .bind(&key.hash.0[..])
    .bind(TokenKind::ApiKey.as_str())
    .bind(key.name)
    .bind(key.scope.as_str())
    .bind(key.made_for.as_str())
    .bind(key.display_prefix)
    .bind(key.expires_at)
    .bind(key.created_at)
    .fetch_one(conn)
    .await
}

/// The operator's API keys, expired ones too until they are pruned, by name.
pub(in crate::access) async fn keys_of(
    db: impl PgExecutor<'_>,
    operator: OperatorId,
) -> Result<Vec<StoredKey>, sqlx::Error> {
    sqlx::query_as(&format!(
        "select {KEY_COLUMNS} from access.api_tokens \
         where kind = 'api_key' and operator_id = $1 order by name"
    ))
    .bind(operator)
    .fetch_all(db)
    .await
}

/// Deletes the operator's API key and returns it; `None` when the operator has no key with
/// this id. Only an `api_key` row is deleted: a login or browser token id is no key.
pub(in crate::access) async fn delete_key(
    conn: &mut PgConnection,
    operator: OperatorId,
    id: ApiKeyId,
) -> Result<Option<StoredKey>, sqlx::Error> {
    sqlx::query_as(&format!(
        "delete from access.api_tokens \
         where id = $1 and operator_id = $2 and kind = 'api_key' returning {KEY_COLUMNS}"
    ))
    .bind(id)
    .bind(operator)
    .fetch_optional(conn)
    .await
}
