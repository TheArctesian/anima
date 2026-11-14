//! Removing the source account keeps its projects or deletes them, as the operator chose
//! (ITG-004); a kept project is adopted by the account that lists its repository next.

use serde_json::json;
use sqlx::PgPool;

use super::github::{Repo, files, list, tree_reads};
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

const CONFIRM: &str = "mutation($id: ID!) { confirmProjectResource(id: $id) { id state } }";

/// PRO-002 outlives the account: account A imported `me/site` and `me/app` first, B lists them
/// too. The operator leaves `me/site` out and confirms a resource of `me/app`, then removes A
/// with its records. B keeps both decisions: `me/site` stays out and is never read again, and
/// the confirmed resource stays.
#[sqlx::test(migrations = "./migrations")]
async fn pro_002_removing_one_account_with_records_keeps_what_another_still_lists(pool: PgPool) {
    let kit = kit(&pool).await;
    files(&kit.stub, "me/site", &[], &[]);
    files(&kit.stub, "me/app", &["Dockerfile"], &[]);
    files(&kit.stub, "me/blog", &[], &[]);
    let repos = [
        Repo::new(1, "me/site"),
        Repo::new(2, "me/app"),
        Repo::new(3, "me/blog"),
    ];
    list(&kit.stub, &repos);
    let first = kit.connect().await;
    kit.run(first).await;
    set_projects(&pool, false).await;
    let second = kit.connect_installation("88").await;
    kit.run(second).await;

    let site = kit.project("site").await["repositories"][0]["id"].clone();
    let left = kit.leave_out(site.as_str().unwrap(), true).await;
    assert!(left["errors"].is_null(), "{left}");
    let server = kit.project("app").await["resources"][0]["id"].clone();
    let confirmed = kit.post(CONFIRM, json!({ "id": server })).await;
    assert_eq!(
        confirmed["data"]["confirmProjectResource"]["state"], "CONFIRMED",
        "{confirmed}"
    );

    let removed = kit
        .post(REMOVE, json!({ "id": first, "delete": true }))
        .await;
    assert!(removed["errors"].is_null(), "{removed}");
    let trees = tree_reads(&kit.stub, "me/site");
    let mut pushed = Repo::new(1, "me/site");
    pushed.pushed_at = "2030-05-03T10:00:00Z".to_owned();
    list(&kit.stub, &[pushed, repos[1].clone(), repos[2].clone()]);
    kit.run(second).await;

    assert_eq!(tree_reads(&kit.stub, "me/site"), trees, "no tree request");
    let out = kit.projects(true).await;
    assert_eq!(slugs(&out), ["site"], "still left out");
    assert_eq!(out[0]["repositories"][0]["id"], site, "the same repository");
    let app = kit.project("app").await;
    assert_eq!(app["resources"][0]["id"], server);
    assert_eq!(app["resources"][0]["state"], "CONFIRMED");
    assert_eq!(slugs(&kit.projects(false).await), ["app", "blog"]);
}
