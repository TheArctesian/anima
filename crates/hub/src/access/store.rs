//! The only SQL for `access.operators`, `access.api_tokens` and `access.login_attempts`.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor};
use uuid::Uuid;

use super::tokens::{Scope, TokenHash};
use crate::ids::OperatorId;

/// An operator account as stored. No `Debug`: it holds the password hash.
#[derive(FromRow)]
pub(super) struct Account {
    pub id: OperatorId,
    pub username: String,
    pub password_hash: String,
    /// Whether a TOTP secret is enrolled; the sealed secret itself is read only by T6's code.
    pub totp_enabled: bool,
}

const ACCOUNT_COLUMNS: &str =
    "id, username, password_hash, totp_secret_enc is not null as totp_enabled";

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

/// Creates the operator, or replaces the password of the one with this username. Returns
/// its id.
pub(super) async fn put_operator(
    conn: &mut PgConnection,
    username: &str,
    password_hash: &str,
    now: DateTime<Utc>,
) -> Result<OperatorId, sqlx::Error> {
    sqlx::query_scalar(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1, $2, $3, $4) \
         on conflict (username) do update set password_hash = excluded.password_hash \
         returning id",
    )
    .bind(Uuid::now_v7())
    .bind(username)
    .bind(password_hash)
    .bind(now)
    .fetch_one(conn)
    .await
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

/// A token row to insert.
pub(super) struct NewToken {
    pub operator: OperatorId,
    pub hash: TokenHash,
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
         (id, operator_id, token_hash, scope, expires_at, created_at) \
         values ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(token.operator)
    .bind(&token.hash.0[..])
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
    pub scope: String,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub(super) async fn token_by_hash(
    db: impl PgExecutor<'_>,
    hash: TokenHash,
) -> Result<Option<StoredToken>, sqlx::Error> {
    sqlx::query_as(
        "select t.id, t.operator_id, o.username, t.scope, t.expires_at, t.last_used_at \
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

/// Deletes the token. Returns false if it was already gone.
pub(super) async fn delete_token(conn: &mut PgConnection, id: Uuid) -> Result<bool, sqlx::Error> {
    let done = sqlx::query("delete from access.api_tokens where id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Records a login attempt as failed until `finish_attempt` says otherwise, so concurrent
/// attempts count it at once (D-8). Returns its id.
pub(super) async fn start_attempt(
    db: impl PgExecutor<'_>,
    at: DateTime<Utc>,
    username: &str,
    client: IpAddr,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query(
        "insert into access.login_attempts (id, at, username, client_addr, ok) \
         values ($1, $2, $3, $4, false)",
    )
    .bind(id)
    .bind(at)
    .bind(username)
    .bind(client)
    .execute(db)
    .await?;
    Ok(id)
}

/// How a login attempt ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Signed in.
    Ok,
    /// Turned away by the throttle before any password check.
    Refused,
}

pub(super) async fn finish_attempt(
    db: impl PgExecutor<'_>,
    id: Uuid,
    outcome: Outcome,
) -> Result<(), sqlx::Error> {
    sqlx::query("update access.login_attempts set ok = $2, refused = $3 where id = $1")
        .bind(id)
        .bind(outcome == Outcome::Ok)
        .bind(outcome == Outcome::Refused)
        .execute(db)
        .await?;
    Ok(())
}

/// Times of the checked failures in `(since, until]` other than attempt `except`, oldest
/// first: those of `username` and those of `client`.
pub(super) async fn failures(
    conn: &mut PgConnection,
    attempt: &AttemptKey<'_>,
    since: DateTime<Utc>,
) -> Result<(Vec<DateTime<Utc>>, Vec<DateTime<Utc>>), sqlx::Error> {
    const BY_USERNAME: &str = "select at from access.login_attempts \
        where username = $1 and not ok and not refused and at > $2 and at <= $3 and id <> $4 \
        order by at";
    const BY_CLIENT: &str = "select at from access.login_attempts \
        where client_addr = $1 and not ok and not refused and at > $2 and at <= $3 and id <> $4 \
        order by at";
    let by_username = sqlx::query_scalar(BY_USERNAME)
        .bind(attempt.username)
        .bind(since)
        .bind(attempt.at)
        .bind(attempt.id)
        .fetch_all(&mut *conn)
        .await?;
    let by_client = sqlx::query_scalar(BY_CLIENT)
        .bind(attempt.client)
        .bind(since)
        .bind(attempt.at)
        .bind(attempt.id)
        .fetch_all(&mut *conn)
        .await?;
    Ok((by_username, by_client))
}

/// The attempt whose throttle state is read.
pub(super) struct AttemptKey<'a> {
    pub id: Uuid,
    pub at: DateTime<Utc>,
    pub username: &'a str,
    pub client: IpAddr,
}
