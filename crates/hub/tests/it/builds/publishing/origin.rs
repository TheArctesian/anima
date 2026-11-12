//! The hub checks the bytes the bucket itself holds, after every upload URL of the job expired,
//! and copies the stable name only from the object it checked (BLD-064, BLD-066, D-28). The
//! public address only has to answer 200.

use std::sync::Arc;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    fake_r2::Object,
    setup::{
        Held, LINUX, LINUX_FEED, LINUX_STABLE, Publishing, linux_feed, linux_files, ready,
        until_waiting, uploaded_linux,
    },
};
use crate::{builds::harness::data, hub::start};

/// The object of the job's `AppImage`.
const APP: &str = "linux-x86_64/0.1.12/Anima_0.1.12_amd64.AppImage";

/// Publishes once; the job failed with a reason that holds `why`, and nothing was written for
/// people.
async fn fails(publishing: &Publishing, held: &Held, why: &str) {
    assert_eq!(publishing.sweep(ready()).await, 1);
    let (state, reason) = publishing.job(held).await;
    assert_eq!(state, "failed");
    let reason = reason.unwrap();
    assert!(reason.contains(why), "{reason}");
    assert!(publishing.fake.object(LINUX_STABLE).is_none());
    assert!(publishing.fake.object(LINUX_FEED).is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn d_28_a_job_is_taken_only_after_its_upload_urls_expired(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let expired = start() + TimeDelta::minutes(15);
    assert_eq!(publishing.sweep(expired).await, 0);
    assert_eq!(publishing.sweep(expired + TimeDelta::seconds(119)).await, 0);
    assert_eq!(publishing.job(&held).await.0, "uploading");
    assert_eq!(publishing.sweep(expired + TimeDelta::minutes(2)).await, 1);
    assert_eq!(publishing.job(&held).await.0, "published");
}

/// The review's case: a builder writes the `AppImage` again while the hub copies it. Its URL has
/// expired by then, so the write is refused and the stable name gets the checked bytes; nor can
/// the URLs write after the publish.
#[sqlx::test(migrations = "./migrations")]
async fn d_28_no_upload_url_writes_once_the_hub_checks_the_files(pool: PgPool) {
    let publishing = Arc::new(Publishing::start(&pool).await);
    publishing.release("0.1.12", "LINUX_X86_64").await;
    let held = publishing.uploading("pop-os", LINUX).await;
    let files = linux_files("0.1.12");
    let feed = linux_feed(&publishing, "0.1.12", &files[0]);
    let granted = data(publishing.grant(&held, &files).await);
    let urls = granted["buildUploadUrls"].as_array().unwrap().clone();
    for (upload, file) in urls.iter().zip(&files) {
        assert_eq!(publishing.put(upload, &file.bytes).await, 200);
    }
    data(publishing.report(&held, &files, &feed).await);
    publishing
        .fake
        .with(|store| store.hang = Some(LINUX_STABLE.to_owned()));
    let running = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    until_waiting(&publishing).await;
    let evil = vec![b'X'; files[0].bytes.len()];
    assert_eq!(publishing.put(&urls[0], &evil).await, 403);
    publishing.fake.with(|store| store.gate.notify_waiters());
    assert_eq!(running.await.unwrap(), 1);
    assert_eq!(publishing.job(&held).await.0, "published");
    assert_eq!(
        publishing.fake.object(LINUX_STABLE).unwrap().bytes,
        files[0].bytes
    );
    let sums = vec![b'0'; files[1].bytes.len()];
    assert_eq!(publishing.put(&urls[1], &sums).await, 403);
}

/// Bytes written again while the URL still worked are the bytes the hub checks.
#[sqlx::test(migrations = "./migrations")]
async fn d_28_bytes_written_again_before_the_urls_expire_fail_the_job(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    publishing.release("0.1.12", "LINUX_X86_64").await;
    let held = publishing.uploading("pop-os", LINUX).await;
    let files = linux_files("0.1.12");
    let feed = linux_feed(&publishing, "0.1.12", &files[0]);
    let granted = data(publishing.grant(&held, &files).await);
    let urls = granted["buildUploadUrls"].as_array().unwrap().clone();
    for (upload, file) in urls.iter().zip(&files) {
        assert_eq!(publishing.put(upload, &file.bytes).await, 200);
    }
    data(publishing.report(&held, &files, &feed).await);
    publishing.at(start() + TimeDelta::minutes(14));
    let evil = vec![b'X'; files[0].bytes.len()];
    assert_eq!(publishing.put(&urls[0], &evil).await, 200);
    fails(&publishing, &held, "does not have the SHA-256").await;
}

/// `CopyObject` names the `ETag` the hub read; an object changed after its check is not copied.
#[sqlx::test(migrations = "./migrations")]
async fn d_28_a_file_changed_after_its_check_is_not_copied(pool: PgPool) {
    let publishing = Arc::new(Publishing::start(&pool).await);
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    publishing
        .fake
        .with(|store| store.hang = Some(LINUX_STABLE.to_owned()));
    let running = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    until_waiting(&publishing).await;
    publishing.fake.with(|store| {
        let app = store.objects.get_mut(APP).unwrap();
        app.bytes[0] ^= 1;
        store.gate.notify_waiters();
    });
    assert_eq!(running.await.unwrap(), 1);
    let (state, reason) = publishing.job(&held).await;
    assert_eq!(state, "failed");
    assert!(reason.unwrap().contains("changed in the release bucket"));
    assert!(publishing.fake.object(LINUX_STABLE).is_none());
    assert!(publishing.fake.object(LINUX_FEED).is_none());
}

/// A cache at the public address may still serve the bytes it saw first; the hub checks the
/// bucket's own.
#[sqlx::test(migrations = "./migrations")]
async fn d_28_a_warm_cache_does_not_hide_the_bytes_in_the_bucket(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, files) = uploaded_linux(&publishing, "0.1.12").await;
    publishing.fake.with(|store| {
        store.stale = Some((APP.to_owned(), files[0].bytes.clone()));
        let app = store.objects.get_mut(APP).unwrap();
        *app = Object::new(vec![b'X'; files[0].bytes.len()]);
    });
    fails(&publishing, &held, "does not have the SHA-256").await;
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_064_a_file_people_cannot_read_fails_the_job(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let sums = "linux-x86_64/0.1.12/SHA256SUMS".to_owned();
    publishing
        .fake
        .with(|store| store.public_missing = Some(sums));
    fails(&publishing, &held, "at the public address").await;
}

/// D-25: when the answer to the `latest.json` write is lost, the hub reads the object back; the
/// write landed, so the job is published.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_feed_write_whose_answer_was_lost_is_read_back(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    publishing
        .fake
        .with(|store| store.lose = Some(LINUX_FEED.to_owned()));
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await.0, "published");
    assert!(publishing.fake.object(LINUX_FEED).is_some());
}

/// The hub copies by `ETag` only, so a file the bucket gives none for fails the job.
#[sqlx::test(migrations = "./migrations")]
async fn d_28_a_file_without_an_etag_fails_the_job(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    publishing
        .fake
        .with(|store| store.no_etag = Some(APP.to_owned()));
    fails(&publishing, &held, "gave no ETag").await;
}

/// URLs granted again under a clock that went back do not shorten the wait: the latest expiry
/// stays.
#[sqlx::test(migrations = "./migrations")]
async fn d_28_the_latest_upload_url_expiry_is_kept(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    publishing.release("0.1.12", "LINUX_X86_64").await;
    let held = publishing.uploading("pop-os", LINUX).await;
    let files = linux_files("0.1.12");
    data(publishing.grant(&held, &files).await);
    publishing.at(start() - TimeDelta::minutes(5));
    let feed = linux_feed(&publishing, "0.1.12", &files[0]);
    data(publishing.upload(&held, &files, &feed).await);
    let early = start() + TimeDelta::minutes(12);
    assert_eq!(publishing.sweep(early).await, 0);
    assert_eq!(publishing.job(&held).await.0, "uploading");
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await.0, "published");
}
