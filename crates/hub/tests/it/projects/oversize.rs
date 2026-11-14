//! A reply larger than the hub reads, or a tree GitHub cut short, is the problem of one
//! repository: the run goes on to the next one and the account stays healthy (SRC-006,
//! SRC-007, PRO-003). Only the listing, without which nothing can be imported, fails the run.

use anima_hub::{engine::sources::Outcome, ids::SourceAccountId};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::github::{Repo, files, list, reads_of, tree_reads, truncated_files};
use super::{Kit, kit};
use crate::sources::last_run;

/// The problems of the newest run, as `(code, subject)`.
async fn problems(pool: &PgPool, account: SourceAccountId) -> Vec<(String, String)> {
    let stored: Value = sqlx::query_scalar(
        "SELECT problems FROM sources.sync_runs WHERE account_id = $1 \
         ORDER BY started_at DESC, id DESC LIMIT 1",
    )
    .bind(account)
    .fetch_one(pool)
    .await
    .unwrap();
    stored
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["code"].as_str().unwrap().into(),
                p["subject"].as_str().unwrap().into(),
            )
        })
        .collect()
}

/// The account's health and failure count as the client reads them.
async fn health(kit: &Kit) -> Value {
    let response = kit
        .post("{ sourceAccounts { health failureCount } }", json!({}))
        .await;
    response["data"]["sourceAccounts"][0].clone()
}

/// The kinds of the project's resources, sorted.
async fn kinds(kit: &Kit, slug: &str) -> Vec<String> {
    let project = kit.project(slug).await;
    let mut kinds: Vec<String> = project["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|resource| resource["kind"].as_str().unwrap().to_owned())
        .collect();
    kinds.sort();
    kinds
}

#[sqlx::test(migrations = "./migrations")]
async fn src_007_an_oversize_tree_is_skipped_until_its_next_push(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    files(&kit.stub, "me/a", &["wrangler.toml"], &[]);
    kit.stub.endless("/repos/me/b/git/trees/main");
    files(&kit.stub, "me/c", &["Dockerfile"], &[]);
    let repos = [
        Repo::new(1, "me/a"),
        Repo::new(2, "me/b"),
        Repo::new(3, "me/c"),
    ];
    list(&kit.stub, &repos);

    assert_eq!(kit.run(id).await, Some(Outcome::Ok));

    assert_eq!(last_run(&pool, id).await.6, None, "no run error");
    assert_eq!(kinds(&kit, "a").await, ["CLOUDFLARE"]);
    assert_eq!(kinds(&kit, "b").await, Vec::<String>::new());
    assert_eq!(kinds(&kit, "c").await, ["SERVER"]);
    assert_eq!(
        problems(&pool, id).await,
        [("tree_too_large".to_owned(), "me/b".to_owned())]
    );
    assert_eq!(
        health(&kit).await,
        json!({ "health": "OK", "failureCount": 0 })
    );
    assert_eq!(reads_of(&kit.stub, "me/b"), 1, "no deployments read for it");

    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(
        tree_reads(&kit.stub, "me/b"),
        1,
        "not read again: no new push"
    );

    let [a, mut b, c] = repos;
    b.pushed_at = "2030-05-03T10:00:00Z".to_owned();
    list(&kit.stub, &[a, b, c]);
    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(tree_reads(&kit.stub, "me/b"), 2, "read again after a push");
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_003_a_truncated_tree_keeps_its_signs_and_is_read_until_its_next_push(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    truncated_files(&kit.stub, "me/big", &["wrangler.toml"]);
    files(&kit.stub, "me/c", &["Dockerfile"], &[]);
    list(&kit.stub, &[Repo::new(1, "me/big"), Repo::new(3, "me/c")]);

    assert_eq!(kit.run(id).await, Some(Outcome::Ok));

    assert_eq!(
        kinds(&kit, "big").await,
        ["CLOUDFLARE"],
        "the part that came"
    );
    assert_eq!(kinds(&kit, "c").await, ["SERVER"]);
    assert_eq!(
        problems(&pool, id).await,
        [("tree_truncated".to_owned(), "me/big".to_owned())]
    );
    assert_eq!(
        health(&kit).await,
        json!({ "health": "OK", "failureCount": 0 })
    );
    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(tree_reads(&kit.stub, "me/big"), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn src_007_an_oversize_listing_still_fails_the_run(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    kit.stub.endless("/installation/repositories");

    assert_eq!(kit.run(id).await, Some(Outcome::Failed));

    assert_eq!(
        last_run(&pool, id).await.6.as_deref(),
        Some("The source sent a reply larger than the hub reads.")
    );
    assert_eq!(health(&kit).await["failureCount"], 1);
    assert_eq!(kit.projects(false).await, Vec::<Value>::new());
}
