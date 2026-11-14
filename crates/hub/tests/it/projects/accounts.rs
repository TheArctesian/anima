//! Removing the source account keeps its projects or deletes them, as the operator chose
//! (ITG-004); a kept project is adopted by the account that lists its repository next.

use serde_json::json;
use sqlx::PgPool;

use super::github::{Repo, list};
use super::{kit, slugs};
use crate::sources::{last_run, set_projects};

const REMOVE: &str = "mutation($id: ID!, $delete: Boolean!) { removeSourceAccount(id: $id, deleteRecords: $delete) }";

async fn rows(pool: &PgPool) -> (i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM projects.projects), (SELECT count(*) FROM projects.repositories)",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn itg_004_remove_with_records_kept_keeps_every_project(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    list(
        &kit.stub,
        &[Repo::new(1, "me/site"), Repo::new(2, "me/blog")],
    );
    kit.run(id).await;
    let removed = kit.post(REMOVE, json!({ "id": id, "delete": false })).await;
    assert!(removed["errors"].is_null(), "{removed}");
    assert_eq!(rows(&pool).await, (2, 2));
    assert_eq!(slugs(&kit.projects(false).await), ["blog", "site"]);

    // Connected again, the same repositories are the same projects, not new ones. Projects
    // is off while it connects, so only the test's run reads them.
    set_projects(&pool, false).await;
    let again = kit.connect().await;
    kit.run(again).await;
    assert_eq!(rows(&pool).await, (2, 2));
    let (_, _, inserted, changed, ..) = last_run(&pool, again).await;
    assert_eq!((inserted, changed), (0, 2), "adopted, not imported");
}

#[sqlx::test(migrations = "./migrations")]
async fn itg_004_remove_with_records_deleted_deletes_its_projects(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    list(
        &kit.stub,
        &[Repo::new(1, "me/site"), Repo::new(2, "me/blog")],
    );
    kit.run(id).await;
    let removed = kit.post(REMOVE, json!({ "id": id, "delete": true })).await;
    assert!(removed["errors"].is_null(), "{removed}");
    assert_eq!(rows(&pool).await, (0, 0));
    assert_eq!(slugs(&kit.projects(false).await), Vec::<String>::new());
}
