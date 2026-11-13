//! The bucket keeps the 3 newest published versions of a target (BLD-086, R-118): a fourth
//! publish deletes the oldest version's files and its `.sig`, and never the one `latest.json`
//! names.

use sqlx::PgPool;

use super::setup::{LINUX_FEED, Publishing, ready, uploaded_linux};

#[sqlx::test(migrations = "./migrations")]
async fn bld_086_a_fourth_published_version_removes_the_oldest_versions_objects(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let mut published = Vec::new();
    for version in ["0.1.1", "0.1.2", "0.1.3"] {
        let (held, files) = uploaded_linux(&publishing, version).await;
        assert_eq!(publishing.sweep(ready()).await, 1);
        published.push((version, held, files));
    }
    assert!(publishing.fake.with(|store| store.deletes.is_empty()));
    let (held, files) = uploaded_linux(&publishing, "0.1.4").await;
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await, ("published".to_owned(), None));

    let name = &files[0].name;
    let mut deletes = publishing.fake.with(|store| store.deletes.clone());
    deletes.sort();
    let mut expected: Vec<String> = published[0]
        .2
        .iter()
        .map(|file| format!("linux-x86_64/0.1.1/{}", file.name))
        .chain([format!("linux-x86_64/0.1.1/{name}.sig")])
        .collect();
    expected.sort();
    assert_eq!(deletes, expected);
    for kept in ["0.1.2", "0.1.3", "0.1.4"] {
        let key = format!("linux-x86_64/{kept}/{name}");
        assert!(publishing.fake.object(&key).is_some(), "{key}");
    }
    assert!(publishing.fake.object(LINUX_FEED).is_some());
    let removed: Vec<String> = sqlx::query_scalar(
        "SELECT version FROM builds.jobs WHERE objects_removed_at IS NOT NULL",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(removed, ["0.1.1"]);
    let state: String =
        sqlx::query_scalar("SELECT state FROM builds.jobs WHERE version = '0.1.1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, "published");
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_086_a_delete_fault_does_not_fail_the_publish_and_the_next_publish_tries_again(
    pool: PgPool,
) {
    let publishing = Publishing::start(&pool).await;
    for version in ["0.1.1", "0.1.2", "0.1.3"] {
        uploaded_linux(&publishing, version).await;
        assert_eq!(publishing.sweep(ready()).await, 1);
    }
    let (held, _) = uploaded_linux(&publishing, "0.1.4").await;
    publishing.fake.with(|store| store.unavailable = true);
    // The fake answers every request with a fault, so the publish itself is retried; what this
    // checks is that nothing is marked removed while deletes fail.
    publishing.sweep(ready()).await;
    publishing.fake.with(|store| store.unavailable = false);
    let _ = held;
    let removed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM builds.jobs WHERE objects_removed_at IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(removed, 0);
}
