//! The `checks` schema: checks, partitioned heartbeats, uptime aggregates and the partition
//! functions (CHK-009, DB-004, DB-005, DB-008, DB-009, DB-011).

mod cascade;
mod columns;
mod parents;
mod partitions;
mod privileges;
mod retention;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// The "Ping" check of `pve-01` in the K6 fixtures.
const PVE_01_PING: &str = "0190a000-0000-7000-8003-000000000001";
const UPTIME_TABLES: [&str; 3] = ["uptime_minute", "uptime_hour", "uptime_day"];

fn id(text: &str) -> Uuid {
    text.parse().unwrap()
}

/// An RFC 3339 instant.
fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

/// The SQLSTATE of a database error, or the error text for any other error.
fn code_of(error: &sqlx::Error) -> String {
    let code = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    code.map_or_else(|| error.to_string(), Into::into)
}

fn constraint_of(error: &sqlx::Error) -> Option<&str> {
    error.as_database_error().and_then(|e| e.constraint())
}

async fn execute(conn: &mut PgConnection, sql: &str) {
    sqlx::query(sql).execute(conn).await.unwrap();
}

/// Runs `sql` and returns its SQLSTATE, or `None` if it succeeded.
async fn refusal(conn: &mut PgConnection, sql: &str) -> Option<String> {
    let error = sqlx::query(sql).execute(conn).await.err()?;
    Some(code_of(&error))
}

/// Runs `body` on `conn` as `anima_runtime`, then resets the role. `body` returns its findings
/// instead of asserting, so the reset runs before the caller asserts.
async fn as_runtime<T>(
    conn: &mut PgConnection,
    body: impl AsyncFnOnce(&mut PgConnection) -> T,
) -> T {
    execute(conn, "set role anima_runtime").await;
    let findings = body(conn).await;
    execute(conn, "reset role").await;
    findings
}

async fn table_exists(conn: &mut PgConnection, name: &str) -> bool {
    sqlx::query_scalar("select to_regclass($1) is not null")
        .bind(name)
        .fetch_one(conn)
        .await
        .unwrap()
}

async fn ensure(conn: &mut PgConnection, instant: DateTime<Utc>) -> Result<(), sqlx::Error> {
    sqlx::query("select checks.ensure_day_partition($1)")
        .bind(instant)
        .execute(conn)
        .await
        .map(|_| ())
}

/// Inserts an Up heartbeat and returns the partition that holds it.
async fn beat(
    conn: &mut PgConnection,
    check: Uuid,
    instant: DateTime<Utc>,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar(
        "insert into checks.heartbeats (check_id, at, status, response_time_ms) \
         values ($1, $2, 'up', 12) returning tableoid::regclass::text",
    )
    .bind(check)
    .bind(instant)
    .fetch_one(conn)
    .await
}

/// Inserts a check with only the columns that have no default, plus an optional push token.
async fn insert_check(
    pool: &PgPool,
    (check, device): (Uuid, &str),
    (kind, name): (&str, &str),
    push_token: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into checks.checks (id, device_id, type, name, push_token) \
         values ($1, $2, $3, $4, $5)",
    )
    .bind(check)
    .bind(id(device))
    .bind(kind)
    .bind(name)
    .bind(push_token)
    .execute(pool)
    .await
    .map(|_| ())
}

async fn count(pool: &PgPool, sql: &str, check: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(check)
        .fetch_one(pool)
        .await
        .unwrap()
}
