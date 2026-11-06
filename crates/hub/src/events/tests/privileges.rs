//! The runtime role may append to the logs but never change or remove an entry, and gets its
//! rights on everything else from migration 0001's default privileges (K2 "Roles", R-016,
//! LOG-006).

use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{day, device_event, new_audit};
use crate::events::{Bus, audit, record};

/// Statements the runtime role must not be able to run against the logs or their partitions.
/// The partitions are those `write_both` creates.
const TAMPERING: [&str; 14] = [
    "update events.audit_log set operator_name = 'someone else'",
    "delete from events.audit_log",
    "truncate events.audit_log",
    "update events.event_log set message = 'changed'",
    "delete from events.event_log",
    "truncate events.event_log",
    "update events.audit_log_20300505 set operator_name = 'someone else'",
    "delete from events.audit_log_20300505",
    "truncate events.audit_log_20300505",
    "drop table events.audit_log_20300505",
    "update events.event_log_20300505 set message = 'changed'",
    "delete from events.event_log_20300505",
    "truncate events.event_log_20300505",
    "drop table events.event_log_20300505",
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

pub(super) async fn execute(conn: &mut PgConnection, sql: &str) {
    sqlx::query(sql).execute(conn).await.unwrap();
}

/// Runs `sql` and returns its SQLSTATE, or `None` if it succeeded.
pub(super) async fn refusal(conn: &mut PgConnection, sql: &str) -> Option<String> {
    let error = sqlx::query(sql).execute(conn).await.err()?;
    let code = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    Some(code.map_or_else(|| error.to_string(), Into::into))
}

/// Runs `body` on `conn` as `anima_runtime`. `conn` must be detached from the pool (R-019), so
/// the role dies with the connection.
pub(super) async fn as_runtime<T>(
    conn: &mut PgConnection,
    body: impl AsyncFnOnce(&mut PgConnection) -> T,
) -> T {
    execute(conn, "set role anima_runtime").await;
    body(conn).await
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
async fn log_006_runtime_role_appends_but_cannot_change_or_remove_entries(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap().detach();

    let (wrote, refusals) = as_runtime(&mut conn, async |conn| {
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
async fn log_006_runtime_role_cannot_run_code_as_the_partition_owner_through_pg_temp(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap().detach();

    let (wrote, injected) = as_runtime(&mut conn, async |conn| {
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

/// The bodies are also schema-qualified, so the attack above needs both defences removed;
/// this pins the search path on its own.
#[sqlx::test(migrations = "./migrations")]
async fn log_006_partition_functions_run_as_owner_with_pg_temp_searched_last(pool: PgPool) {
    let functions: Vec<(String, bool, Vec<String>)> = sqlx::query_as(
        "select p.oid::pg_catalog.regprocedure::pg_catalog.text, p.prosecdef, p.proconfig \
         from pg_catalog.pg_proc p \
         where p.pronamespace = 'events'::pg_catalog.regnamespace order by 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let names: Vec<&str> = functions.iter().map(|(name, ..)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "events.drop_day_partitions_before(text,date,timestamp with time zone)",
            "events.ensure_day_partition(text,timestamp with time zone)",
        ]
    );
    for (name, definer, config) in functions {
        assert!(definer, "{name}");
        assert_eq!(config, ["search_path=pg_catalog, pg_temp"], "{name}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_default_privileges_reach_objects_created_after_migration_0001(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let schema = format!("zz_test_{}", Uuid::now_v7().simple());
    execute(&mut conn, &format!("create schema {schema}")).await;
    execute(&mut conn, &format!("create table {schema}.t (id serial)")).await;
    execute(
        &mut conn,
        &format!("create function {schema}.f() returns int language sql as 'select 1'"),
    )
    .await;
    // PUBLIC gets EXECUTE on every new function; without it only 0001's default grant remains.
    execute(
        &mut conn,
        &format!("revoke execute on function {schema}.f() from public"),
    )
    .await;
    let checks = [
        (
            format!("has_schema_privilege('anima_runtime', '{schema}', 'USAGE')"),
            true,
        ),
        (
            format!("has_table_privilege('anima_runtime', '{schema}.t', 'SELECT')"),
            true,
        ),
        (
            format!("has_table_privilege('anima_runtime', '{schema}.t', 'INSERT')"),
            true,
        ),
        (
            format!("has_table_privilege('anima_runtime', '{schema}.t', 'UPDATE')"),
            true,
        ),
        (
            format!("has_table_privilege('anima_runtime', '{schema}.t', 'DELETE')"),
            true,
        ),
        (
            format!("has_table_privilege('anima_runtime', '{schema}.t', 'TRUNCATE')"),
            false,
        ),
        (
            format!("has_sequence_privilege('anima_runtime', '{schema}.t_id_seq', 'USAGE')"),
            true,
        ),
        (
            format!("has_sequence_privilege('anima_runtime', '{schema}.t_id_seq', 'SELECT')"),
            true,
        ),
        (
            format!("has_function_privilege('anima_runtime', '{schema}.f()', 'EXECUTE')"),
            true,
        ),
        (
            "has_table_privilege('anima_runtime', 'devices.devices', 'UPDATE')".to_owned(),
            true,
        ),
        (
            "has_schema_privilege('anima_runtime', 'devices', 'USAGE')".to_owned(),
            true,
        ),
    ];
    let mut found = Vec::new();
    for (expression, expected) in &checks {
        let held: bool = sqlx::query_scalar(&format!("select {expression}"))
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        found.push((expression.clone(), held, *expected));
    }

    execute(&mut conn, &format!("drop schema {schema} cascade")).await;
    for (expression, held, expected) in found {
        assert_eq!(held, expected, "{expression}");
    }
}
