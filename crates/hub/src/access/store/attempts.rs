//! The SQL for `access.login_attempts`: each sign-in attempt, the throttle's view of them and
//! their deletion when the operator is reset (D-8, D-9).

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::access::throttle::LOOKBACK;

/// Deletes the login attempts of these usernames and every attempt from an address where one
/// of them was tried, so a reset operator can sign in at once, even after typos that locked
/// the address (D-8, D-9).
pub(in crate::access) async fn delete_attempts_of(
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

/// Records a login attempt as failed until `finish_attempt` says otherwise, so concurrent
/// attempts count it at once (D-8). Stores `client` in canonical form, so an IPv4-mapped IPv6
/// address and its IPv4 address are one address to the throttle (D-11). Returns its id.
pub(in crate::access) async fn start_attempt(
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
    .bind(client.to_canonical())
    .execute(db)
    .await?;
    Ok(id)
}

/// How a login attempt ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::access) enum Outcome {
    /// Signed in.
    Ok,
    /// Turned away by the throttle before any password check.
    Refused,
}

pub(in crate::access) async fn finish_attempt(
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

/// Times of the checked failures from `client` that can still throttle an attempt at `at`,
/// those in `(at - LOOKBACK, at]`, oldest first, leaving out `except`: the caller's own
/// attempt, already recorded as failed. Reads `client` in canonical form, as `start_attempt`
/// stores it (D-11). Sign-in and `joinShare` both read through here, so they share one rule
/// (D-8, CR-16).
pub(in crate::access) async fn failures(
    db: impl PgExecutor<'_>,
    client: IpAddr,
    at: DateTime<Utc>,
    except: Option<Uuid>,
) -> Result<Vec<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar(
        "select at from access.login_attempts \
         where client_addr = $1 and not ok and not refused and at > $2 and at <= $3 \
         and ($4::uuid is null or id <> $4) order by at",
    )
    .bind(client.to_canonical())
    .bind(at - LOOKBACK)
    .bind(at)
    .bind(except)
    .fetch_all(db)
    .await
}

/// The attempt whose throttle state is read.
pub(in crate::access) struct AttemptKey {
    pub id: Uuid,
    pub at: DateTime<Utc>,
    pub client: IpAddr,
}
