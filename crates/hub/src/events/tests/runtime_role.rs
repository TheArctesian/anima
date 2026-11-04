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
