//! A job whose files are in ends `published` or `failed`, in the order BLD-064 to BLD-066 set,
//! signed by the hub (R-116), and a publisher that stops is taken over (BLD-063 to BLD-070, D-25,
//! R-108).

use std::time::Duration;

use chrono::TimeDelta;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use super::{
    fake_r2::{Object, hex},
    setup::{
        File, Held, LINUX_FEED, LINUX_STABLE, MACOS, Publishing, ready, until_waiting,
        uploaded_linux,
    },
};
use crate::{builds::harness::data, hub::error};

async fn written(publishing: &Publishing, held: &Held) -> Option<Vec<String>> {
    sqlx::query_scalar("SELECT written FROM builds.feeds WHERE job_id = $1::uuid")
        .bind(&held.job)
        .fetch_one(&publishing.pool)
        .await
        .unwrap()
}

/// An object by its digest, so a failed comparison prints no file.
fn object(bytes: &[u8], cache: &str) -> (String, Option<String>) {
    (hex(&Sha256::digest(bytes)), Some(cache.to_owned()))
}

fn parts(object: Option<Object>) -> Option<(String, Option<String>)> {
    object.map(|o| (hex(&Sha256::digest(&o.bytes)), o.cache_control))
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_066_a_linux_job_publishes_its_sig_the_stable_name_then_latest_json_last(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, files) = uploaded_linux(&publishing, "0.1.12").await;
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await, ("published".to_owned(), None));
    let fake = &publishing.fake;
    for file in &files {
        let key = format!("linux-x86_64/0.1.12/{}", file.name);
        let stored = parts(fake.object(&key));
        assert_eq!(stored, Some(object(&file.bytes, "public, max-age=3600")));
    }
    let stable = parts(fake.object(LINUX_STABLE));
    assert_eq!(stable, Some(object(&files[0].bytes, "no-cache")));
    let feed: Value = serde_json::from_slice(&fake.object(LINUX_FEED).unwrap().bytes).unwrap();
    assert_eq!(feed["version"], "0.1.12");
    assert_eq!(
        fake.object(LINUX_FEED).unwrap().cache_control.as_deref(),
        Some("no-cache")
    );
    let sig = format!("linux-x86_64/0.1.12/{}.sig", files[0].name);
    let writes = fake.with(|store| store.writes.clone());
    assert_eq!(
        writes[writes.len() - 3..],
        [sig.as_str(), LINUX_STABLE, LINUX_FEED]
    );
    publishing.assert_signed("0.1.12", &files[0]);
    assert_eq!(
        fake.object(&sig).unwrap().cache_control.as_deref(),
        Some("public, max-age=3600")
    );
    assert_eq!(
        written(&publishing, &held).await,
        Some(vec!["linux-x86_64".to_owned()])
    );
    let event: String = sqlx::query_scalar(
        "SELECT message FROM events.event_log WHERE message LIKE 'Anima 0.1.12%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(event, "Anima 0.1.12 for linux-x86_64 is published.");
    assert_eq!(publishing.sweep(ready()).await, 0);
}

/// The files of a macOS job: the updater archive, the DMG and the sums.
fn mac_files(publishing: &Publishing) -> (Vec<File>, Value) {
    // The builder proposes no signature; the hub signs (R-116).
    let archive = File::new("Anima.app.tar.gz", "app archive ".repeat(300));
    let dmg = File::new("Anima_0.1.12_universal.dmg", "disk image ".repeat(300));
    let sums = format!(
        "{}  {}\n{}  {}\n",
        archive.input()["sha256"].as_str().unwrap(),
        archive.name,
        dmg.input()["sha256"].as_str().unwrap(),
        dmg.name
    );
    let entry = json!({
        "url": publishing.public("darwin-universal/0.1.12/Anima.app.tar.gz"),
    });
    let feed = json!({
        "version": "0.1.12",
        "platforms": { "darwin-aarch64": entry, "darwin-x86_64": entry },
    });
    (vec![archive, dmg, File::new("SHA256SUMS", sums)], feed)
}

/// BLD-068 until OD-13 (R-108): a macOS job publishes its versioned files and the DMG's stable
/// name, and no feed and no `.sig`.
#[sqlx::test(migrations = "./migrations")]
async fn bld_068_a_macos_job_publishes_without_a_feed(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    publishing.release("0.1.12", "DARWIN_UNIVERSAL").await;
    let held = publishing.uploading("mac-mini", MACOS).await;
    let (files, feed) = mac_files(&publishing);
    data(publishing.upload(&held, &files, &feed).await);
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await.0, "published");
    let fake = &publishing.fake;
    let dmg = parts(fake.object("Anima.dmg"));
    assert_eq!(dmg, Some(object(&files[1].bytes, "no-cache")));
    let feeds = fake.with(|store| {
        store
            .objects
            .keys()
            .filter(|key| key.ends_with("latest.json"))
            .count()
    });
    assert_eq!(feeds, 0);
    assert_eq!(written(&publishing, &held).await, Some(Vec::new()));
    // No feed names the archive, so the hub signs nothing (BLD-068, T6 security review C-2).
    let sig = fake.object("darwin-universal/0.1.12/Anima.app.tar.gz.sig");
    assert!(sig.is_none());
}

/// D-25: a hub that stops after the stable copy, before it sends `latest.json`, leaves the job
/// held; once the hold ends, the next sweep runs every step again and publishes.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_publisher_stopped_between_copy_and_feed_is_taken_over(pool: PgPool) {
    let publishing = std::sync::Arc::new(Publishing::start(&pool).await);
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    publishing
        .fake
        .with(|store| store.hang_answer = Some(LINUX_STABLE.to_owned()));
    let running = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    let copied = async {
        while publishing.fake.object(LINUX_STABLE).is_none() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(10), copied)
        .await
        .unwrap();
    running.abort();
    assert!(publishing.fake.object(LINUX_FEED).is_none());
    assert_eq!(publishing.job(&held).await.0, "uploading");
    publishing.fake.with(|store| store.hang_answer = None);
    assert_eq!(publishing.sweep(ready() + TimeDelta::minutes(5)).await, 0);
    assert_eq!(publishing.sweep(ready() + TimeDelta::minutes(11)).await, 1);
    assert_eq!(publishing.job(&held).await.0, "published");
    assert!(publishing.fake.object(LINUX_FEED).is_some());
}

/// D-25: a try that a later try took over while it read the files writes nothing and changes
/// nothing; only the job's latest try ends it.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_try_that_was_taken_over_does_not_end_the_job(pool: PgPool) {
    let publishing = std::sync::Arc::new(Publishing::start(&pool).await);
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let sums = "linux-x86_64/0.1.12/SHA256SUMS".to_owned();
    publishing.fake.with(|store| store.hang_read = Some(sums));
    let first = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    until_waiting(&publishing).await;
    // A later try takes the job over while the first one waits on the bucket.
    sqlx::query("UPDATE builds.jobs SET publish_attempts = 2 WHERE id = $1::uuid")
        .bind(&held.job)
        .execute(&pool)
        .await
        .unwrap();
    publishing.fake.with(|store| store.gate.notify_waiters());
    assert_eq!(first.await.unwrap(), 1);
    assert_eq!(publishing.job(&held).await.0, "uploading");
    assert!(publishing.fake.object(LINUX_STABLE).is_none());
    assert!(publishing.fake.object(LINUX_FEED).is_none());
    let feed: Option<Vec<String>> =
        sqlx::query_scalar("SELECT written FROM builds.feeds WHERE job_id = $1::uuid")
            .bind(&held.job)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(feed, None);
}

/// D-25, BLD-070: an unreachable bucket is tried 3 times, then the job fails, and the
/// `latest.json` already there stays as it was.
#[sqlx::test(migrations = "./migrations")]
async fn bld_070_an_unreachable_bucket_fails_the_job_after_3_tries(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let old = Object::new(&b"{\"version\":\"0.1.11\"}"[..]);
    publishing.fake.with(|store| {
        store.objects.insert(LINUX_FEED.to_owned(), old.clone());
        store.unavailable = true;
    });
    for minutes in [0, 2, 4] {
        let now = ready() + TimeDelta::minutes(minutes);
        assert_eq!(publishing.sweep(now).await, 1, "{minutes}");
    }
    let (state, reason) = publishing.job(&held).await;
    assert_eq!(state, "failed");
    assert_eq!(
        reason.as_deref(),
        Some("The release bucket could not be reached 3 times.")
    );
    assert_eq!(publishing.fake.object(LINUX_FEED).unwrap().bytes, old.bytes);
}

/// D-25: once the files are in, the builder's lease is over; it can neither report nor get new
/// upload URLs while the hub publishes.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_the_builder_is_refused_once_the_hub_publishes(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, files) = uploaded_linux(&publishing, "0.1.12").await;
    let report = "mutation($id: ID!) { reportBuildJob(id: $id, state: FAILED, logLines: [], \
                  failureReason: \"x\") { id } }";
    let hub = &publishing.hub;
    let refused = hub
        .post(Some(&held.token), report, json!({ "id": held.job }))
        .await;
    assert_eq!(error(&refused).1, "CONFLICT", "{refused}");
    let regrant = publishing.grant(&held, &files).await;
    assert_eq!(error(&regrant).1, "CONFLICT", "{regrant}");
    let lease: Option<String> =
        sqlx::query_scalar("SELECT lease_ends_at::text FROM builds.jobs WHERE id = $1::uuid")
            .bind(&held.job)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(lease, None);
}
