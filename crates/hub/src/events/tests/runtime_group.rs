//! Migration 0001 creates `anima_runtime` as a plain group, or refuses one that has more
//! rights (K2 "Roles", R-016, DB-001).

use sqlx::PgPool;

use super::{privileges::execute, runtime_role::test_role};

/// Migration 0001 as written; tests run it for a stand-in group name.
const MIGRATION_0001: &str = include_str!("../../../migrations/0001_access_runtime_role.sql");

/// Runs migration 0001 with `group` in place of `anima_runtime`, so a test can give the
/// stand-in extra rights without touching the cluster-wide group other tests use. Returns the
/// SQLSTATE, or `None` if it succeeded.
async fn migrate_0001_for(conn: &mut sqlx::PgConnection, group: &str) -> Option<String> {
    let sql = MIGRATION_0001.replace("anima_runtime", group);
    let error = sqlx::raw_sql(&sql).execute(conn).await.err()?;
    let code = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    Some(code.map_or_else(|| error.to_string(), Into::into))
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_migration_0001_refuses_a_runtime_group_with_more_rights_than_a_plain_group(
    pool: PgPool,
) {
    let mut conn = pool.acquire().await.unwrap();
    let parent = test_role("parent");
    execute(&mut conn, &format!("create role {parent} nologin")).await;
    let member_of_parent = format!("nologin in role {parent}");
    let cases = [
        ("login", Some("P0001")),
        ("nologin superuser", Some("P0001")),
        ("nologin createrole", Some("P0001")),
        ("nologin createdb", Some("P0001")),
        ("nologin bypassrls", Some("P0001")),
        (member_of_parent.as_str(), Some("P0001")),
        ("nologin", None),
    ];
    let mut outcomes = Vec::new();
    for (attributes, _) in &cases {
        let group = test_role("group");
        execute(&mut conn, &format!("create role {group} {attributes}")).await;
        outcomes.push(migrate_0001_for(&mut conn, &group).await);
        execute(&mut conn, &format!("drop owned by {group}")).await;
        execute(&mut conn, &format!("drop role {group}")).await;
    }

    execute(&mut conn, &format!("drop role {parent}")).await;
    for ((attributes, expected), outcome) in cases.iter().zip(outcomes) {
        assert_eq!(outcome.as_deref(), *expected, "{attributes}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_migration_0001_creates_a_missing_runtime_group_as_a_plain_group(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let group = test_role("group");

    let outcome = migrate_0001_for(&mut conn, &group).await;
    let attributes: (bool, bool, bool, bool, bool) = sqlx::query_as(
        "select rolcanlogin, rolsuper, rolcreaterole, rolcreatedb, rolbypassrls \
         from pg_catalog.pg_roles where rolname = $1",
    )
    .bind(&group)
    .fetch_one(&mut *conn)
    .await
    .unwrap();

    execute(&mut conn, &format!("drop owned by {group}")).await;
    execute(&mut conn, &format!("drop role {group}")).await;
    assert_eq!(outcome, None);
    assert_eq!(attributes, (false, false, false, false, false));
}
