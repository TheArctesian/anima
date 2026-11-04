//! A group check's children are checks of the same device (CHK-004: a group counts as the
//! worst of its children, inside its device's status).

use sqlx::PgPool;
use uuid::Uuid;

use super::{DOCKER_HOST, PVE_01, constraint_of, insert_check};

async fn set_parent(pool: &PgPool, child: Uuid, parent: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("update checks.checks set parent_id = $1 where id = $2")
        .bind(parent)
        .bind(child)
        .execute(pool)
        .await
        .map(|_| ())
}

async fn parent_of(pool: &PgPool, child: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("select parent_id from checks.checks where id = $1")
        .bind(child)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn chk_004_a_parent_on_another_device_or_the_check_itself_is_refused(pool: PgPool) {
    let (group, child, elsewhere) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    insert_check(&pool, (group, PVE_01), ("group", "All"), None)
        .await
        .unwrap();
    insert_check(&pool, (child, PVE_01), ("tcp", "SSH"), None)
        .await
        .unwrap();
    insert_check(&pool, (elsewhere, DOCKER_HOST), ("tcp", "SSH"), None)
        .await
        .unwrap();

    let across = set_parent(&pool, elsewhere, group).await.unwrap_err();
    let missing = set_parent(&pool, child, Uuid::now_v7()).await.unwrap_err();
    let itself = set_parent(&pool, group, group).await.unwrap_err();
    set_parent(&pool, child, group).await.unwrap();

    assert_eq!(constraint_of(&across), Some("checks_parent_fkey"));
    assert_eq!(constraint_of(&missing), Some("checks_parent_fkey"));
    assert_eq!(constraint_of(&itself), Some("checks_parent_not_self_check"));
    assert_eq!(parent_of(&pool, elsewhere).await, None);
    assert_eq!(parent_of(&pool, child).await, Some(group));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn chk_004_deleting_a_parent_keeps_the_child_on_its_device_without_a_parent(pool: PgPool) {
    let (group, child) = (Uuid::now_v7(), Uuid::now_v7());
    insert_check(&pool, (group, PVE_01), ("group", "All"), None)
        .await
        .unwrap();
    insert_check(&pool, (child, PVE_01), ("tcp", "SSH"), None)
        .await
        .unwrap();
    set_parent(&pool, child, group).await.unwrap();

    sqlx::query("delete from checks.checks where id = $1")
        .bind(group)
        .execute(&pool)
        .await
        .unwrap();

    let device: Uuid = sqlx::query_scalar("select device_id from checks.checks where id = $1")
        .bind(child)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(parent_of(&pool, child).await, None);
    assert_eq!(device, super::id(PVE_01));
}
