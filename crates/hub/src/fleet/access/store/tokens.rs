//! The SQL for `access.api_tokens`: login, browser, API-key and builder tokens, stored only as
//! their hash (D-7, D-23, BLD-023).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor};
use uuid::Uuid;

use crate::{
    fleet::access::tokens::{Scope, TokenHash, TokenKind},
    ids::OperatorId,
};

/// Deletes every token of the operator.
pub(in crate::fleet::access) async fn delete_operator_tokens(
    conn: &mut PgConnection,
    operator: OperatorId,
) -> Result<(), sqlx::Error> {
    sqlx::query("delete from access.api_tokens where operator_id = $1")
        .bind(operator)
        .execute(conn)
        .await?;
    Ok(())
}

/// A login, browser or builder token row to insert; none has a name or a display prefix.
pub(in crate::fleet::access) struct NewToken {
    pub operator: OperatorId,
    pub hash: TokenHash,
    pub kind: TokenKind,
    pub scope: Scope,
    pub expires_at: DateTime<Utc>,
}

/// Inserts the token and returns its id.
pub(in crate::fleet::access) async fn insert_token(
    conn: &mut PgConnection,
    token: &NewToken,
    now: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error> {
    insert_token_with_idle(conn, token, now, None).await
}

/// Inserts the token with the longest time with no request before it is refused, and returns
/// its id. Only the browser session of a hosted hub has such a rule (HMD-062). A token with a
/// rule counts as used at `now`, so that the time with no request starts when it is made.
pub(in crate::fleet::access) async fn insert_token_with_idle(
    conn: &mut PgConnection,
    token: &NewToken,
    now: DateTime<Utc>,
    idle: Option<TimeDelta>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query(
        "insert into access.api_tokens \
         (id, operator_id, token_hash, kind, scope, expires_at, created_at, idle_seconds, \
         last_used_at) values ($1, $2, $3, $4, $5, $6, $7, $8, case when $8 is null then null \
         else $7 end)",
    )
    .bind(id)
    .bind(token.operator)
    .bind(&token.hash.0[..])
    .bind(token.kind.as_str())
    .bind(token.scope.as_str())
    .bind(token.expires_at)
    .bind(now)
    .bind(idle.and_then(|idle| i32::try_from(idle.num_seconds()).ok()))
    .execute(conn)
    .await?;
    Ok(id)
}

/// A stored token with its operator's name.
#[derive(FromRow)]
pub(in crate::fleet::access) struct StoredToken {
    pub id: Uuid,
    pub operator_id: OperatorId,
    pub username: String,
    pub kind: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    /// The seconds with no request after which the token is refused; `None` for no such rule.
    pub idle_seconds: Option<i32>,
    /// An API key's name and what it is made for; `None` for the other kinds (LOG-015).
    pub name: Option<String>,
    pub made_for: Option<String>,
}

pub(in crate::fleet::access) async fn token_by_hash(
    db: impl PgExecutor<'_>,
    hash: TokenHash,
) -> Result<Option<StoredToken>, sqlx::Error> {
    sqlx::query_as(
        "select t.id, t.operator_id, o.username, t.kind, t.scope, t.expires_at, t.last_used_at, \
         t.idle_seconds, t.name, t.made_for from access.api_tokens t join access.operators o on o.id = t.operator_id \
         where t.token_hash = $1",
    )
    .bind(&hash.0[..])
    .fetch_optional(db)
    .await
}

pub(in crate::fleet::access) async fn touch_token(
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
pub(in crate::fleet::access) async fn token_exists(
    db: impl PgExecutor<'_>,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("select exists (select from access.api_tokens where id = $1)")
        .bind(id)
        .fetch_one(db)
        .await
}

/// Deletes the token. Returns false if it was already gone.
pub(in crate::fleet::access) async fn delete_token(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query("delete from access.api_tokens where id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Deletes the token if it is a builder token, so no other token is revoked through a builder.
/// Returns false if there was no such builder token.
pub(in crate::fleet::access) async fn delete_builder_token(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let done =
        sqlx::query("delete from access.api_tokens where id = $1 and kind = $2 and scope = $3")
            .bind(id)
            .bind(TokenKind::Builder.as_str())
            .bind(Scope::Builder.as_str())
            .execute(conn)
            .await?;
    Ok(done.rows_affected() == 1)
}
