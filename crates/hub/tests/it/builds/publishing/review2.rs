//! The security re-review's cases (T3 round 2), as regression tests: once a feed write was sent
//! the job's files stay and a later try confirms the feed (D-25); a try writes only while its
//! hold outlasts the write; and a checked file that changed after publishing is reported
//! (D-28).

use std::sync::Arc;

use anima_hub::{
    clock::Clock,
    fleet::{
        builds::{self, Updates},
        events::Bus,
    },
};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{
    fake_r2::Object,
    setup::{
        LINUX, LINUX_FEED, LINUX_STABLE, Publishing, linux_feed, linux_files, ready, until_waiting,
        uploaded_linux,
    },
};
use crate::{builds::harness::data, hub::start};

/// The job's `AppImage` object.
const APP: &str = "linux-x86_64/0.1.12/Anima_0.1.12_amd64.AppImage";

async fn set_attempts(publishing: &Publishing, job: &str, attempts: i32) {
    sqlx::query("UPDATE builds.jobs SET publish_attempts = $2 WHERE id = $1::uuid")
        .bind(job)
        .bind(attempts)
        .execute(&publishing.pool)
        .await
        .unwrap();
}

/// The warnings in the event log.
async fn warnings(publishing: &Publishing) -> Vec<String> {
    sqlx::query_scalar("SELECT message FROM events.event_log WHERE severity = 'warning'")
        .fetch_all(&publishing.pool)
        .await
        .unwrap()
}

/// The review's case: on the last try the feed write lands, but its answer and the read-back
/// are both lost. The job fails as unknown, and the files the feed names stay.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_job_that_fails_after_its_feed_was_sent_keeps_its_files(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    set_attempts(&publishing, &held.job, 2).await;
    publishing.fake.with(|store| {
        store.lose = Some(LINUX_FEED.to_owned());
        store.tamper = Some(LINUX_FEED.to_owned());
    });
    assert_eq!(publishing.sweep(ready()).await, 1);
    let (state, reason) = publishing.job(&held).await;
    assert_eq!(state, "failed");
    assert_eq!(
        reason.as_deref(),
        Some("Feed state unknown, check latest.json.")
    );
    let feed = publishing.fake.object(LINUX_FEED).unwrap();
    assert!(String::from_utf8_lossy(&feed.bytes).contains("0.1.12"));
    assert!(
        publishing.fake.object(APP).is_some(),
        "the feed's file stays"
    );
    assert!(publishing.fake.with(|store| store.deletes.is_empty()));
}

/// A try whose feed write and read-back were both lost leaves the job; the next try reads the
/// feed that landed and publishes without writing it again.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_later_try_publishes_a_sent_feed_that_landed(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    publishing.fake.with(|store| {
        store.lose = Some(LINUX_FEED.to_owned());
        store.tamper = Some(LINUX_FEED.to_owned());
    });
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await.0, "uploading");
    publishing.fake.with(|store| {
        store.lose = None;
        store.tamper = None;
    });
    assert_eq!(publishing.sweep(ready() + TimeDelta::minutes(2)).await, 1);
    assert_eq!(publishing.job(&held).await.0, "published");
    let writes = publishing.fake.with(|store| store.writes.clone());
    let feeds = writes.iter().filter(|key| *key == LINUX_FEED).count();
    assert_eq!(feeds, 1, "the feed is confirmed, not written again");
}

/// The review's case: a try's feed write is in flight when a later try takes the job. The later
/// try finds another feed, so the job fails, and the files stay for the write that lands after.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_feed_in_flight_past_a_takeover_names_files_that_stay(pool: PgPool) {
    let publishing = Arc::new(Publishing::start(&pool).await);
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let old = Object::new(&b"{\"version\":\"0.1.11\"}"[..]);
    publishing.fake.with(|store| {
        store.objects.insert(LINUX_FEED.to_owned(), old.clone());
        store.hang = Some(LINUX_FEED.to_owned());
    });
    let first = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    until_waiting(&publishing).await;
    publishing.fake.with(|store| store.hang = None);
    assert_eq!(publishing.sweep(ready() + TimeDelta::minutes(11)).await, 1);
    let (state, reason) = publishing.job(&held).await;
    assert_eq!(state, "failed");
    assert!(reason.unwrap().contains("holds another feed"));
    publishing.fake.with(|store| store.gate.notify_waiters());
    first.await.unwrap();
    assert_eq!(publishing.job(&held).await.0, "failed");
    assert_ne!(publishing.fake.object(LINUX_FEED).unwrap().bytes, old.bytes);
    assert!(
        publishing.fake.object(APP).is_some(),
        "the feed's file stays"
    );
    assert!(publishing.fake.with(|store| store.deletes.is_empty()));
}

/// A try writes only while its hold outlasts a write: with a minute of hold left after its
/// reads, it copies nothing.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_try_writes_only_while_its_hold_outlasts_the_write(pool: PgPool) {
    let publishing = Arc::new(Publishing::start(&pool).await);
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let sums = "linux-x86_64/0.1.12/SHA256SUMS".to_owned();
    publishing.fake.with(|store| store.hang_read = Some(sums));
    let running = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    until_waiting(&publishing).await;
    publishing.at(ready() + TimeDelta::minutes(9));
    publishing.fake.with(|store| {
        store.hang_read = None;
        store.gate.notify_waiters();
    });
    assert_eq!(running.await.unwrap(), 1);
    assert_eq!(publishing.job(&held).await.0, "uploading");
    assert!(publishing.fake.object(LINUX_STABLE).is_none());
    assert!(publishing.fake.object(LINUX_FEED).is_none());
}

/// One publisher sweep with the hub's clock at `hub` and R2's at `r2`.
async fn sweep_apart(publishing: &Publishing, hub: DateTime<Utc>, r2: DateTime<Utc>) -> usize {
    publishing.clock.set(hub);
    publishing.fake.with(|store| store.now = Some(r2));
    let (bus, updates) = (Bus::new(16), Updates::new());
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let clock: &dyn Clock = publishing.clock.as_ref();
    let queue = publishing.queue(&bus, &updates);
    builds::publish_ready(queue, &publishing.bucket, &http, clock)
        .await
        .unwrap()
}

/// The review's case: the hub's clock runs 3 minutes ahead of R2's, so a builder's URL still
/// works while the hub publishes and rewrites the `AppImage`. The hub reads the checked files'
/// `ETag`s again after the feed and warns.
#[sqlx::test(migrations = "./migrations")]
async fn d_28_a_file_changed_after_its_check_is_reported(pool: PgPool) {
    let publishing = Arc::new(Publishing::start(&pool).await);
    publishing.release("0.1.12", "LINUX_X86_64").await;
    let held = publishing.uploading("pop-os", LINUX).await;
    let files = linux_files("0.1.12");
    publishing.at(start());
    let granted = data(publishing.grant(&held, &files).await);
    let urls = granted["buildUploadUrls"].as_array().unwrap().clone();
    for (upload, file) in urls.iter().zip(&files) {
        assert_eq!(publishing.put(upload, &file.bytes).await, 200);
    }
    let feed = linux_feed(&publishing, "0.1.12", &files[0]);
    data(publishing.report(&held, &files, &feed).await);
    publishing
        .fake
        .with(|store| store.hang = Some(LINUX_FEED.to_owned()));
    let hub = start() + TimeDelta::minutes(17);
    let running = {
        let publishing = publishing.clone();
        let r2 = hub - TimeDelta::minutes(3);
        tokio::spawn(async move { sweep_apart(&publishing, hub, r2).await })
    };
    until_waiting(&publishing).await;
    let evil = vec![b'X'; files[0].bytes.len()];
    assert_eq!(publishing.put(&urls[0], &evil).await, 200);
    publishing.fake.with(|store| store.gate.notify_waiters());
    assert_eq!(running.await.unwrap(), 1);
    assert_eq!(publishing.job(&held).await.0, "published");
    let heads = publishing.fake.with(|store| store.heads.clone());
    assert_eq!(heads, [APP, "linux-x86_64/0.1.12/SHA256SUMS"]);
    let warned = warnings(&publishing).await;
    assert_eq!(warned.len(), 1, "{warned:?}");
    assert!(
        warned[0].contains("Anima_0.1.12_amd64.AppImage changed"),
        "{}",
        warned[0]
    );
    assert!(!warned[0].contains("SHA256SUMS"));
}

/// A publish whose files did not change after the check warns of nothing.
#[sqlx::test(migrations = "./migrations")]
async fn d_28_an_unchanged_publish_warns_of_nothing(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await.0, "published");
    assert_eq!(publishing.fake.with(|store| store.heads.len()), 2);
    assert_eq!(warnings(&publishing).await, Vec::<String>::new());
}
