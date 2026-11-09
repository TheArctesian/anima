//! The only SQL for `access.operators`, `access.api_tokens` (API keys in `store/keys.rs`),
//! `access.login_attempts` (in `store/attempts.rs`) and `access.permitted_networks` (in
//! `store/permitted.rs`).

mod attempts;
mod keys;
mod permitted;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor};
use uuid::Uuid;

pub(super) use attempts::{
    AttemptKey, Outcome, delete_attempts_of, failures, failures_from, finish_attempt, start_attempt,
};
pub(super) use keys::{KEY_NAME_INDEX, NewKey, StoredKey, delete_key, insert_key, keys_of};
pub(super) use permitted::{permitted_networks, replace_labelled_networks};

use super::tokens::{Scope, TokenHash, TokenKind};
use crate::ids::OperatorId;

/// An operator account as stored. No `Debug`: it holds the password hash.
#[derive(FromRow)]
pub(super) struct Account {
    pub id: OperatorId,
    pub username: String,
    pub password_hash: String,
    /// The TOTP secret sealed with the Vault (`Sealed::to_bytes`), if one is enrolled.
    pub totp_secret_enc: Option<Vec<u8>>,
    /// The last TOTP step that signed in; a code must belong to a later one.
    pub totp_last_step: Option<i64>,
}

const ACCOUNT_COLUMNS: &str = "id, username, password_hash, totp_secret_enc, totp_last_step";

pub(super) async fn account_by_username(
    db: impl PgExecutor<'_>,
    username: &str,
) -> Result<Option<Account>, sqlx::Error> {
    sqlx::query_as(&format!(
        "select {ACCOUNT_COLUMNS} from access.operators where username = $1"
    ))
    .bind(username)
    .fetch_optional(db)
    .await
}

pub(super) async fn account_by_id(
    db: impl PgExecutor<'_>,
    id: OperatorId,
) -> Result<Option<Account>, sqlx::Error> {
    sqlx::query_as(&format!(
        "select {ACCOUNT_COLUMNS} from access.operators where id = $1"
    ))
    .bind(id)
    .fetch_optional(db)
    .await
}

/// Makes the one operator account (OD-2, D-9) have this username and hash: updates the
/// existing row, or inserts the first one. Returns its id and its previous username.
pub(super) async fn replace_operator(
    conn: &mut PgConnection,
    username: &str,
    password_hash: &str,
    now: DateTime<Utc>,
) -> Result<(OperatorId, Option<String>), sqlx::Error> {
    // `for update` holds the one row until commit, so a concurrent `operator set` waits and
    // then updates the same account instead of reading a stale username. On an empty table it
    // locks nothing: a racing first insert fails on `operators_single`.
    let existing: Option<(OperatorId, String)> =
        sqlx::query_as("select id, username from access.operators for update")
            .fetch_optional(&mut *conn)
            .await?;
    if let Some((id, previous)) = existing {
        sqlx::query("update access.operators set username = $2, password_hash = $3 where id = $1")
            .bind(id)
            .bind(username)
            .bind(password_hash)
            .execute(&mut *conn)
            .await?;
        return Ok((id, Some(previous)));
    }
    let id = sqlx::query_scalar(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1, $2, $3, $4) returning id",
    )
    .bind(Uuid::now_v7())
    .bind(username)
    .bind(password_hash)
    .bind(now)
    .fetch_one(&mut *conn)
    .await?;
    Ok((id, None))
}

/// Stores (`Some`) or removes (`None`) the operator's sealed TOTP secret and forgets the
/// last used step.
pub(super) async fn set_totp(
    db: impl PgExecutor<'_>,
    operator: OperatorId,
    sealed: Option<&[u8]>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "update access.operators set totp_secret_enc = $2, totp_last_step = null where id = $1",
    )
    .bind(operator)
    .bind(sealed)
    .execute(db)
    .await?;
    Ok(())
}

/// Locks the operator row until the transaction ends and tells whether it still has the
/// username, password hash and TOTP secret of `account`, as they were checked.
pub(super) async fn lock_unchanged(
    conn: &mut PgConnection,
    account: &Account,
) -> Result<bool, sqlx::Error> {
    let found: Option<bool> = sqlx::query_scalar(
        "select true from access.operators where id = $1 and username = $2 \
         and password_hash = $3 and totp_secret_enc is not distinct from $4 for update",
    )
    .bind(account.id)
    .bind(&account.username)
    .bind(&account.password_hash)
    .bind(account.totp_secret_enc.as_deref())
    .fetch_optional(conn)
    .await?;
    Ok(found.is_some())
}

/// Marks `step` used if it is later than the last used one and the operator still has the
/// secret that was checked. One atomic update, so of two sign-ins with one code only one
/// claims the step. Returns whether this call claimed it.
pub(super) async fn claim_totp_step(
    db: impl PgExecutor<'_>,
    operator: OperatorId,
    sealed: &[u8],
    step: i64,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query(
        "update access.operators set totp_last_step = $3 \
         where id = $1 and totp_secret_enc = $2 \
         and (totp_last_step is null or totp_last_step < $3)",
    )
    .bind(operator)
    .bind(sealed)
    .bind(step)
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// What [`prune`] deletes: attempts at or before `attempts`, login and browser tokens that
/// expired at or before `tokens`, and API keys that expired at or before `keys`.
pub(super) struct Cutoffs {
    pub attempts: DateTime<Utc>,
    pub tokens: DateTime<Utc>,
    pub keys: DateTime<Utc>,
}

/// Deletes the attempts and tokens past their cutoffs (K2 retention). Returns how many
/// attempts, then how many tokens of any kind.
pub(super) async fn prune(
    conn: &mut PgConnection,
    cutoffs: &Cutoffs,
) -> Result<(u64, u64), sqlx::Error> {
    let attempts = sqlx::query("delete from access.login_attempts where at <= $1")
        .bind(cutoffs.attempts)
        .execute(&mut *conn)
        .await?;
    let tokens = sqlx::query(
        "delete from access.api_tokens where (kind <> 'api_key' and expires_at <= $1) \
         or (kind = 'api_key' and expires_at <= $2)",
    )
    .bind(cutoffs.tokens)
    .bind(cutoffs.keys)
    .execute(&mut *conn)
    .await?;
    Ok((attempts.rows_affected(), tokens.rows_affected()))
}

/// Deletes every token of the operator.
pub(super) async fn delete_operator_tokens(
    conn: &mut PgConnection,
    operator: OperatorId,
) -> Result<(), sqlx::Error> {
    sqlx::query("delete from access.api_tokens where operator_id = $1")
        .bind(operator)
        .execute(conn)
        .await?;
    Ok(())
}

/// A login or browser token row to insert; neither has a name or a display prefix.
pub(super) struct NewToken {
    pub operator: OperatorId,
    pub hash: TokenHash,
    pub kind: TokenKind,
    pub scope: Scope,
    pub expires_at: DateTime<Utc>,
}

/// Inserts the token and returns its id.
pub(super) async fn insert_token(
    conn: &mut PgConnection,
    token: &NewToken,
    now: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query(
        "insert into access.api_tokens \
         (id, operator_id, token_hash, kind, scope, expires_at, created_at) \
         values ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(token.operator)
    .bind(&token.hash.0[..])
    .bind(token.kind.as_str())
    .bind(token.scope.as_str())
    .bind(token.expires_at)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(id)
}

/// A stored token with its operator's name.
#[derive(FromRow)]
pub(super) struct StoredToken {
    pub id: Uuid,
    pub operator_id: OperatorId,
    pub username: String,
    pub kind: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub(super) async fn token_by_hash(
    db: impl PgExecutor<'_>,
    hash: TokenHash,
) -> Result<Option<StoredToken>, sqlx::Error> {
    sqlx::query_as(
        "select t.id, t.operator_id, o.username, t.kind, t.scope, t.expires_at, t.last_used_at \
         from access.api_tokens t join access.operators o on o.id = t.operator_id \
         where t.token_hash = $1",
    )
    .bind(&hash.0[..])
    .fetch_optional(db)
    .await
}

pub(super) async fn touch_token(
    db: impl PgExecutor<'_>,
    id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("update access.api_tokens set last_used_at = $2 where id = $1")
        .bind(id)
        .bind(now)
        .execute(db)
        .await?;
    Ok(())
}

/// Whether the token is still stored: `logout`, `operator set` and `prune` delete tokens.
pub(super) async fn token_exists(db: impl PgExecutor<'_>, id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("select exists (select from access.api_tokens where id = $1)")
        .bind(id)
        .fetch_one(db)
        .await
}

/// Deletes the token. Returns false if it was already gone.
pub(super) async fn delete_token(conn: &mut PgConnection, id: Uuid) -> Result<bool, sqlx::Error> {
    let done = sqlx::query("delete from access.api_tokens where id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() == 1)
}
