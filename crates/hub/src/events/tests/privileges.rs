//! The hub role may append to the logs but never change or remove an entry (LOG-006, K2).

use serde_json::json;
use sqlx::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use super::{day, device_event, new_audit};
use crate::events::{Bus, audit, record};

/// Statements a writer must not be able to run against the audit log or its partitions.
const TAMPERING: [&str; 6] = [
    "update events.audit_log set operator_name = 'someone else'",
    "delete from events.audit_log",
    "truncate events.audit_log",
    "update events.audit_log_20300505 set operator_name = 'someone else'",
    "delete from events.audit_log_20300505",
    "drop table events.audit_log_20300505",
];

/// Temporary objects that shadow every type name the partition function could resolve through
/// `pg_temp`; each domain's CHECK records that it ran as the function owner.
const SHADOWED_TYPES: [&str; 7] = [
    "date",
    "text",
    "timestamptz",
    "regclass",
    "int4",
    "bool",
    "varchar",
];

async fn execute(conn: &mut PgConnection, sql: &str) {
    sqlx::query(sql).execute(conn).await.unwrap();
}

/// Runs `sql` and returns its SQLSTATE, or `None` if it succeeded.
async fn refusal(conn: &mut PgConnection, sql: &str) -> Option<String> {
    let error = sqlx::query(sql).execute(conn).await.err()?;
    let code = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    Some(code.map_or_else(|| error.to_string(), Into::into))
}

/// Runs `body` on `conn` as a fresh member of `anima_events_writer`, then drops the role and
/// everything it owns. `body` returns its findings instead of asserting, so the cleanup runs
/// on every path before the caller asserts.
async fn as_events_writer<T>(
    conn: &mut PgConnection,
    body: impl AsyncFnOnce(&mut PgConnection) -> T,
) -> T {
    let role = format!("anima_test_{}", Uuid::now_v7().simple());
    execute(
        conn,
        &format!("create role {role} nologin in role anima_events_writer"),
    )
    .await;
    execute(conn, &format!("set role {role}")).await;
    let findings = body(conn).await;
    execute(conn, "reset role").await;
    execute(conn, &format!("drop owned by {role}")).await;
    execute(conn, &format!("drop role {role}")).await;
    findings
}

/// Records one event and one audit entry on a day without partitions.
async fn write_both(conn: &mut PgConnection) -> Result<(), String> {
    let at = day(2030, 5, 5);
    record(conn, &Bus::new(1), at, device_event("as hub"))
        .await
        .map_err(|error| format!("record: {error:?}"))?;
    audit(conn, at, new_audit(json!({ "name": "x" })))
        .await
        .map_err(|error| format!("audit: {error:?}"))
}

#[sqlx::test(migrations = "./migrations")]
async fn log_006_events_writer_appends_but_cannot_change_or_remove_entries(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();

    let (wrote, refusals) = as_events_writer(&mut conn, async |conn| {
        let wrote = write_both(conn).await;
        let mut refusals = Vec::new();
        for sql in TAMPERING {
            refusals.push((sql, refusal(conn, sql).await));
        }
        (wrote, refusals)
    })
    .await;

    assert_eq!(wrote, Ok(()));
    for (sql, code) in refusals {
        assert_eq!(code.as_deref(), Some("42501"), "{sql}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_006_writer_cannot_run_code_as_the_partition_owner_through_pg_temp(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();

    let (wrote, injected) = as_events_writer(&mut conn, async |conn| {
        execute(conn, "create temp table injected (via text)").await;
        execute(
            conn,
            "create function pg_temp.evil(via pg_catalog.text) returns pg_catalog.bool \
             language sql as $$ insert into pg_temp.injected values (via); select true $$",
        )
        .await;
        for name in SHADOWED_TYPES {
            let domain = format!(
                "create domain pg_temp.{name} as pg_catalog.{name} check (pg_temp.evil('{name}'))"
            );
            execute(conn, &domain).await;
        }
        let wrote = write_both(conn).await;
        let injected: Vec<String> = sqlx::query_scalar("select via from pg_temp.injected")
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        (wrote, injected)
    })
    .await;

    assert_eq!(wrote, Ok(()));
    assert_eq!(injected, Vec::<String>::new(), "code ran as the owner");
}

/// The body is also schema-qualified, so the attack above needs both defences removed; this
/// pins the search path on its own.
#[sqlx::test(migrations = "./migrations")]
async fn log_006_partition_function_runs_as_owner_with_pg_temp_searched_last(pool: PgPool) {
    let (definer, config): (bool, Vec<String>) = sqlx::query_as(
        "select prosecdef, proconfig from pg_catalog.pg_proc \
         where oid = 'events.ensure_day_partition(text, timestamptz)'::pg_catalog.regprocedure",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(definer);
    assert_eq!(config, ["search_path=pg_catalog, pg_temp"]);
}
