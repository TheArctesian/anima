//! The start check refuses a hub role that could change the audit log (K2 "Roles", R-016,
//! LOG-006).

use sqlx::PgPool;
use uuid::Uuid;

use super::{pool_as, privileges::execute};
use crate::events::{Error, check_runtime_role};

/// Runs the start check on a session whose role is `role`.
async fn check_as(pool: &PgPool, role: &str) -> Result<(), Error> {
    let session = pool_as(pool, role).await;
    let outcome = check_runtime_role(&session).await;
    session.close().await;
    outcome
}

fn test_role(purpose: &str) -> String {
    format!("anima_test_{}_{purpose}", Uuid::now_v7().simple())
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_superuser(pool: PgPool) {
    let outcome = check_runtime_role(&pool).await;

    assert!(matches!(outcome, Err(Error::Superuser)), "{outcome:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_role_that_can_become_a_superuser(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let superuser: String = sqlx::query_scalar("select current_user::text")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    let role = test_role("climber");
    execute(
        &mut conn,
        &format!("create role {role} nologin in role \"{superuser}\""),
    )
    .await;

    let outcome = check_as(&pool, &role).await;

    execute(&mut conn, &format!("drop role {role}")).await;
    assert!(matches!(outcome, Err(Error::Superuser)), "{outcome:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_the_audit_log_owner_and_its_members(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let (owner, member) = (test_role("owner"), test_role("member"));
    execute(&mut conn, &format!("create role {owner} nologin")).await;
    execute(
        &mut conn,
        &format!("create role {member} nologin in role {owner}"),
    )
    .await;
    execute(
        &mut conn,
        &format!("alter table events.audit_log owner to {owner}"),
    )
    .await;

    let as_owner = check_as(&pool, &owner).await;
    let as_member = check_as(&pool, &member).await;

    execute(
        &mut conn,
        "alter table events.audit_log owner to current_user",
    )
    .await;
    execute(&mut conn, &format!("drop role {member}")).await;
    execute(&mut conn, &format!("drop role {owner}")).await;
    assert!(matches!(as_owner, Err(Error::OwnsAuditLog)), "{as_owner:?}");
    assert!(
        matches!(as_member, Err(Error::OwnsAuditLog)),
        "{as_member:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_passes_a_plain_member_of_anima_runtime(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let role = test_role("hub");
    execute(
        &mut conn,
        &format!("create role {role} nologin in role anima_runtime"),
    )
    .await;

    let outcome = check_as(&pool, &role).await;

    execute(&mut conn, &format!("drop role {role}")).await;
    assert!(outcome.is_ok(), "{outcome:?}");
}

#[test]
fn api_010_start_check_errors_are_plain_sentences() {
    for error in [Error::Superuser, Error::OwnsAuditLog] {
        let message = error.to_string();
        assert!(message.starts_with("The hub's database role"), "{message}");
        assert!(message.ends_with('.'), "{message}");
    }
}

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
