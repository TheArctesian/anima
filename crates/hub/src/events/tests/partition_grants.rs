//! Day partitions get their privileges from migration 0502 alone, never from default
//! privileges, also on an installation upgraded from 0500 (K2 "Roles", R-016, R-023, LOG-006).

use sqlx::{PgConnection, PgPool};

use super::{privileges::execute, test_role};

/// Migration 0502 as written; the upgrade test runs it again.
const MIGRATION_0502: &str =
    include_str!("../../../migrations/0502_events_partition_privileges.sql");

/// Every grant on the named partitions to a role other than their owner, as
/// `(partition, grantee, privilege)`, sorted.
async fn grants_on(conn: &mut PgConnection, partitions: &[&str]) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "select c.relname::text, a.grantee::regrole::text, a.privilege_type \
         from pg_catalog.pg_class c cross join lateral pg_catalog.aclexplode(c.relacl) a \
         where c.relname = any($1) and a.grantee <> c.relowner \
         order by 1, 2, 3",
    )
    .bind(partitions)
    .fetch_all(conn)
    .await
    .unwrap()
}

/// SELECT and INSERT for `anima_runtime` on each partition, as `grants_on` lists them.
fn select_and_insert(partitions: &[&str]) -> Vec<(String, String, String)> {
    partitions
        .iter()
        .flat_map(|table| {
            ["INSERT", "SELECT"].map(|privilege| {
                (
                    (*table).to_owned(),
                    "anima_runtime".to_owned(),
                    privilege.to_owned(),
                )
            })
        })
        .collect()
}

/// A new partition's privileges come from `events.ensure_day_partition` alone: default
/// privileges for another role or PUBLIC, global or in `events`, never reach it (0502, R-023).
#[sqlx::test(migrations = "./migrations")]
async fn r_023_new_partitions_get_select_and_insert_for_anima_runtime_only(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let group = test_role("group");
    let defaults = [
        format!("update, delete on tables to {group}"),
        "truncate, references on tables to public".to_owned(),
    ];
    execute(&mut conn, &format!("create role {group} nologin")).await;
    for grant in &defaults {
        execute(
            &mut conn,
            &format!("alter default privileges grant {grant}"),
        )
        .await;
    }
    execute(
        &mut conn,
        &format!(
            "alter default privileges in schema events grant trigger, maintain on tables to {group}"
        ),
    )
    .await;
    for log in ["event_log", "audit_log"] {
        execute(
            &mut conn,
            &format!("select events.ensure_day_partition('{log}', '2030-06-06 12:00+00')"),
        )
        .await;
    }
    let partitions = ["audit_log_20300606", "event_log_20300606"];
    let grants = grants_on(&mut conn, &partitions).await;
    // The owner keeps its own grants on the partition, so it can still maintain it directly.
    let owner_grants: i64 = sqlx::query_scalar(
        "select count(*) \
         from pg_catalog.pg_class c cross join lateral pg_catalog.aclexplode(c.relacl) a \
         where c.relname in ('event_log_20300606', 'audit_log_20300606') \
             and a.grantee = c.relowner and a.privilege_type = 'DELETE'",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();

    execute(&mut conn, &format!("drop owned by {group}")).await;
    execute(&mut conn, &format!("drop role {group}")).await;
    execute(
        &mut conn,
        "alter default privileges revoke truncate, references on tables from public",
    )
    .await;
    assert_eq!(grants, select_and_insert(&partitions));
    assert_eq!(owner_grants, 2);
}

/// A partition created before 0502 may carry grants that 0500's function kept from default
/// privileges, including a chain where a grantee passed its right on with GRANT OPTION (C5);
/// 0502 resets it once, like a new partition.
#[sqlx::test(migrations = "./migrations")]
async fn r_023_migration_0502_resets_partitions_created_before_it(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let (group, chained) = (test_role("group"), test_role("chained"));
    let partitions = ["audit_log_20300707", "event_log_20300707"];
    execute(&mut conn, &format!("create role {group} nologin")).await;
    execute(&mut conn, &format!("create role {chained} nologin")).await;
    execute(
        &mut conn,
        &format!("grant usage on schema events to {group}"),
    )
    .await;
    for log in ["event_log", "audit_log"] {
        execute(
            &mut conn,
            &format!("select events.ensure_day_partition('{log}', '2030-07-07 12:00+00')"),
        )
        .await;
    }
    for partition in partitions {
        let leftovers = format!(
            "grant update, delete on events.{partition} to {group}; \
             grant truncate, references on events.{partition} to public; \
             grant update, trigger on events.{partition} to anima_runtime; \
             grant select on events.{partition} to {group} with grant option; \
             set role {group}; \
             grant select on events.{partition} to {chained}; \
             reset role"
        );
        sqlx::raw_sql(&leftovers).execute(&mut *conn).await.unwrap();
    }

    sqlx::raw_sql(MIGRATION_0502)
        .execute(&mut *conn)
        .await
        .unwrap();
    let grants = grants_on(&mut conn, &partitions).await;

    for role in [&chained, &group] {
        execute(&mut conn, &format!("drop owned by {role} cascade")).await;
        execute(&mut conn, &format!("drop role {role}")).await;
    }
    assert_eq!(grants, select_and_insert(&partitions));
}
