//! The start check refuses a hub role that could change the logs (K2 "Roles", R-016,
//! LOG-006).

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{pool_as, privileges::execute};
use crate::events::{RoleError, check_runtime_role};

/// Runs the start check on a session whose role is `role`.
async fn check_as(pool: &PgPool, role: &str) -> Result<(), RoleError> {
    let session = pool_as(pool, role).await;
    let outcome = check_runtime_role(&session).await;
    session.close().await;
    outcome
}

pub(super) fn test_role(purpose: &str) -> String {
    format!("anima_test_{}_{purpose}", Uuid::now_v7().simple())
}

/// Runs `sql`, which may hold several statements, with every `{role}` replaced by `role`.
async fn run_for(conn: &mut PgConnection, sql: &str, role: &str) {
    sqlx::raw_sql(&sql.replace("{role}", role))
        .execute(conn)
        .await
        .unwrap();
}

/// Creates both logs' partitions for 2030-05-05 and takes USAGE on schema `events` away from
/// `anima_runtime`, so the check must work from the catalog alone.
async fn partitions_without_schema_usage(conn: &mut PgConnection) {
    for log in ["event_log", "audit_log"] {
        execute(
            conn,
            &format!("select events.ensure_day_partition('{log}', '2030-05-05 12:00+00')"),
        )
        .await;
    }
    execute(conn, "revoke usage on schema events from anima_runtime").await;
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_superuser(pool: PgPool) {
    let outcome = check_runtime_role(&pool).await;

    assert!(matches!(outcome, Err(RoleError::Superuser)), "{outcome:?}");
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
    assert!(matches!(outcome, Err(RoleError::Superuser)), "{outcome:?}");
}

/// Owning a parent or a partition of either log, or being a member of its owner, is refused.
#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_an_owner_of_a_log_or_partition_and_its_members(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    partitions_without_schema_usage(&mut conn).await;
    let tables = [
        "events.audit_log",
        "events.event_log",
        "events.audit_log_20300505",
        "events.event_log_20300505",
    ];
    let mut outcomes = Vec::new();
    for table in tables {
        let (owner, member) = (test_role("owner"), test_role("member"));
        execute(&mut conn, &format!("create role {owner} nologin")).await;
        execute(
            &mut conn,
            &format!("create role {member} nologin in role {owner}"),
        )
        .await;
        execute(&mut conn, &format!("alter table {table} owner to {owner}")).await;
        let as_owner = check_as(&pool, &owner).await;
        let as_member = check_as(&pool, &member).await;
        execute(
            &mut conn,
            &format!("alter table {table} owner to current_user"),
        )
        .await;
        execute(&mut conn, &format!("drop role {member}")).await;
        execute(&mut conn, &format!("drop owned by {owner}")).await;
        execute(&mut conn, &format!("drop role {owner}")).await;
        outcomes.push((table, as_owner, as_member));
    }

    for (table, as_owner, as_member) in outcomes {
        assert!(
            matches!(as_owner, Err(RoleError::OwnsLogs)),
            "{table}: {as_owner:?}"
        );
        assert!(
            matches!(as_member, Err(RoleError::OwnsLogs)),
            "{table}: {as_member:?}"
        );
    }
}

/// The owner of schema `events` may drop any table in it, so a member of that owner is
/// refused even when it owns no table.
#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_member_of_the_events_schema_owner(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let (owner, member) = (test_role("schema_owner"), test_role("member"));
    execute(&mut conn, &format!("create role {owner} nologin")).await;
    execute(
        &mut conn,
        &format!("create role {member} nologin in role anima_runtime, {owner}"),
    )
    .await;
    execute(&mut conn, &format!("alter schema events owner to {owner}")).await;

    let outcome = check_as(&pool, &member).await;

    execute(&mut conn, "alter schema events owner to current_user").await;
    execute(&mut conn, &format!("drop role {member}")).await;
    execute(&mut conn, &format!("drop owned by {owner}")).await;
    execute(&mut conn, &format!("drop role {owner}")).await;
    assert!(
        matches!(outcome, Err(RoleError::OwnsEventsSchema)),
        "{outcome:?}"
    );
}

/// Each case gives a plain member of `anima_runtime` one right that could change or remove
/// log entries (setup), then takes it away again (teardown).
const CHANGE_RIGHTS: [(&str, &str); 10] = [
    ("grant pg_write_all_data to {role}", ""),
    ("grant update on events.audit_log_20300505 to {role}", ""),
    (
        "grant update (operator_name) on events.audit_log_20300505 to {role}",
        "",
    ),
    ("grant delete on events.event_log_20300505 to {role}", ""),
    ("grant truncate on events.audit_log to {role}", ""),
    ("grant trigger on events.event_log to {role}", ""),
    ("grant create on schema events to {role}", ""),
    (
        "grant delete on events.audit_log_20300505 to public",
        "revoke delete on events.audit_log_20300505 from public",
    ),
    (
        "create role {role}_w nologin; \
         grant update on events.event_log_20300505 to {role}_w; \
         grant {role}_w to {role} with inherit false",
        "drop owned by {role}_w; drop role {role}_w",
    ),
    (
        "create role {role}_w nologin; \
         grant pg_write_all_data to {role}_w; \
         grant {role}_w to {role} with inherit false, set true",
        "drop role {role}_w",
    ),
];

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_role_that_can_change_log_entries(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    partitions_without_schema_usage(&mut conn).await;
    let mut outcomes = Vec::new();
    for (setup, teardown) in CHANGE_RIGHTS {
        let role = test_role("writer");
        execute(
            &mut conn,
            &format!("create role {role} nologin in role anima_runtime"),
        )
        .await;
        run_for(&mut conn, setup, &role).await;
        outcomes.push((setup, check_as(&pool, &role).await));
        run_for(&mut conn, teardown, &role).await;
        execute(&mut conn, &format!("drop owned by {role}")).await;
        execute(&mut conn, &format!("drop role {role}")).await;
    }

    for (setup, outcome) in outcomes {
        assert!(
            matches!(outcome, Err(RoleError::CanChangeLogs)),
            "{setup}: {outcome:?}"
        );
    }
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

    let with_usage = check_as(&pool, &role).await;
    partitions_without_schema_usage(&mut conn).await;
    let from_catalog = check_as(&pool, &role).await;

    execute(&mut conn, &format!("drop role {role}")).await;
    assert!(with_usage.is_ok(), "{with_usage:?}");
    assert!(from_catalog.is_ok(), "{from_catalog:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_database_without_the_events_schema(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    execute(&mut conn, "drop schema events cascade").await;

    let outcome = check_runtime_role(&pool).await;

    assert!(
        matches!(outcome, Err(RoleError::NotMigrated)),
        "{outcome:?}"
    );
}

#[test]
fn api_010_start_check_errors_are_plain_sentences() {
    let refusals = [
        RoleError::Superuser,
        RoleError::OwnsLogs,
        RoleError::OwnsEventsSchema,
        RoleError::CanChangeLogs,
    ];
    for error in refusals {
        let message = error.to_string();
        assert!(message.starts_with("The hub's database role"), "{message}");
        assert!(message.ends_with('.'), "{message}");
    }
    for message in [
        RoleError::NotMigrated.to_string(),
        RoleError::Database(sqlx::Error::PoolTimedOut).to_string(),
    ] {
        assert!(message.starts_with("The "), "{message}");
        assert!(message.ends_with('.'), "{message}");
    }
}
