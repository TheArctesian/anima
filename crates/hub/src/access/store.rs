//! The only SQL for `access.operators`, `access.api_tokens`, `access.login_attempts` and
//! `access.permitted_networks`.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use ipnet::IpNet;
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

/// Deletes the login attempts of these usernames and every attempt from an address where one
/// of them was tried, so a reset operator can sign in at once, even after typos that locked
/// the address (D-8, D-9).
pub(super) async fn delete_attempts_of(
    conn: &mut PgConnection,
    usernames: &[&str],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "delete from access.login_attempts where username = any($1) or client_addr in \
         (select client_addr from access.login_attempts where username = any($1))",
    )
    .bind(usernames)
    .execute(conn)
    .await?;
    Ok(())
}

/// Deletes login attempts at or before `cutoff` and tokens expired at `now` (K2 retention).
/// Returns how many of each.
pub(super) async fn prune(
    conn: &mut PgConnection,
    cutoff: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(u64, u64), sqlx::Error> {
    let attempts = sqlx::query("delete from access.login_attempts where at <= $1")
        .bind(cutoff)
        .execute(&mut *conn)
        .await?;
    let tokens = sqlx::query("delete from access.api_tokens where expires_at <= $1")
        .bind(now)
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

/// Times of the checked failures from the attempt's client address in `(since, attempt.at]`,
/// other than the attempt itself, oldest first.
pub(super) async fn failures(
    conn: &mut PgConnection,
    attempt: &AttemptKey,
    since: DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar(
        "select at from access.login_attempts \
         where client_addr = $1 and not ok and not refused and at > $2 and at <= $3 \
         and id <> $4 order by at",
    )
    .bind(attempt.client)
    .bind(since)
    .bind(attempt.at)
    .bind(attempt.id)
    .fetch_all(conn)
    .await
}

/// The attempt whose throttle state is read.
pub(super) struct AttemptKey {
    pub id: Uuid,
    pub at: DateTime<Utc>,
    pub client: IpAddr,
}

/// Makes the networks labelled `label` exactly `networks`: deletes the others with that label
/// and inserts the missing ones. A network stored under another label is left as it is.
pub(super) async fn replace_labelled_networks(
    conn: &mut PgConnection,
    label: &str,
    networks: &[IpNet],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "delete from access.permitted_networks where label = $1 and cidr <> all($2::cidr[])",
    )
    .bind(label)
    .bind(networks)
    .execute(&mut *conn)
    .await?;
    let ids: Vec<Uuid> = networks.iter().map(|_| Uuid::now_v7()).collect();
    sqlx::query(
        "insert into access.permitted_networks (id, cidr, label) \
         select id, cidr, $3 from unnest($1::uuid[], $2::cidr[]) as given (id, cidr) \
         on conflict (cidr) do nothing",
    )
    .bind(ids)
    .bind(networks)
    .bind(label)
    .execute(conn)
    .await?;
    Ok(())
}

/// Every permitted network.
pub(super) async fn permitted_networks(db: impl PgExecutor<'_>) -> Result<Vec<IpNet>, sqlx::Error> {
    sqlx::query_scalar("select cidr from access.permitted_networks")
        .fetch_all(db)
        .await
}
