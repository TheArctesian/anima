//! Every job the hub publishes ends, and only the try that holds it writes (D-25): a try taken
//! over writes no feed, each signed request carries the time it was sent, spent tries fail the
//! job, a hub with publishing off fails what it was given, and the operator cannot cancel a job
//! the hub publishes. The database keeps the publishing columns' rules.

use std::sync::Arc;

use anima_hub::{
    builds::{self, Updates},
    events::Bus,
};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{
    fake_r2::Object,
    setup::{LINUX_FEED, LINUX_STABLE, Publishing, ready, until_waiting, uploaded_linux},
};
use crate::hub::error;

/// The review's case: a try waits on the bucket after its copy while a later try takes the job
/// over and fails it. The first try checks its hold before the feed and writes nothing, so the
/// `latest.json` people read stays as it was (BLD-070).
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_taken_over_try_does_not_write_the_feed_of_a_failed_job(pool: PgPool) {
    let publishing = Arc::new(Publishing::start(&pool).await);
    let (held, files) = uploaded_linux(&publishing, "0.1.12").await;
    let old = Object::new(&b"{\"version\":\"0.1.11\"}"[..]);
    publishing.fake.with(|store| {
        store.objects.insert(LINUX_FEED.to_owned(), old.clone());
        store.hang_answer = Some(LINUX_STABLE.to_owned());
    });
    let first = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    until_waiting(&publishing).await;
    let key = format!("linux-x86_64/0.1.12/{}", files[0].name);
    publishing.fake.with(|store| store.tamper = Some(key));
    assert_eq!(publishing.sweep(ready() + TimeDelta::minutes(11)).await, 1);
    assert_eq!(publishing.job(&held).await.0, "failed");
    publishing.fake.with(|store| {
        store.tamper = None;
        store.gate.notify_waiters();
    });
    first.await.unwrap();
    assert_eq!(publishing.job(&held).await.0, "failed");
    assert_eq!(publishing.fake.object(LINUX_FEED).unwrap().bytes, old.bytes);
}

/// A try that waits minutes on the bucket signs its next request with the time it sends it,
/// which the bucket checks against its own clock.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_each_signed_request_reads_the_clock_again(pool: PgPool) {
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
    publishing.at(ready() + TimeDelta::minutes(6));
    publishing.fake.with(|store| store.gate.notify_waiters());
    assert_eq!(running.await.unwrap(), 1);
    assert_eq!(publishing.job(&held).await, ("published".to_owned(), None));
    assert!(publishing.fake.object(LINUX_FEED).is_some());
}

/// A try still running when its hold ran out writes nothing: another try may hold the job now.
/// Here the same sweep takes the job again, and that second try publishes it.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_try_past_its_hold_writes_nothing(pool: PgPool) {
    let publishing = Arc::new(Publishing::start(&pool).await);
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let sums = "linux-x86_64/0.1.12/SHA256SUMS".to_owned();
    publishing.fake.with(|store| store.hang_read = Some(sums));
    let running = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    until_waiting(&publishing).await;
    publishing.at(ready() + TimeDelta::minutes(11));
    publishing.fake.with(|store| {
        store.hang_read = None;
        store.gate.notify_waiters();
    });
    assert_eq!(running.await.unwrap(), 2);
    assert_eq!(publishing.job(&held).await.0, "published");
    let tries: i32 =
        sqlx::query_scalar("SELECT publish_attempts FROM builds.jobs WHERE id = $1::uuid")
            .bind(&held.job)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(tries, 2);
    let writes = publishing.fake.with(|store| store.writes.clone());
    let stable = writes.iter().filter(|key| *key == LINUX_STABLE).count();
    assert_eq!(stable, 1, "only the second try copies");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_job_whose_tries_are_spent_fails(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    sqlx::query("UPDATE builds.jobs SET publish_attempts = 3 WHERE id = $1::uuid")
        .bind(&held.job)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(publishing.sweep(ready()).await, 1);
    let (state, reason) = publishing.job(&held).await;
    assert_eq!(state, "failed");
    assert_eq!(
        reason.as_deref(),
        Some("Publishing stopped 3 times without an end.")
    );
    assert!(publishing.fake.object(LINUX_STABLE).is_none());
    assert!(publishing.fake.object(LINUX_FEED).is_none());
}

/// A try that was taken over and then refuses a file does not fail the job: only the try that
/// holds the job ends it.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_taken_over_try_that_refuses_does_not_fail_the_job(pool: PgPool) {
    let publishing = Arc::new(Publishing::start(&pool).await);
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let sums = "linux-x86_64/0.1.12/SHA256SUMS".to_owned();
    publishing
        .fake
        .with(|store| store.hang_read = Some(sums.clone()));
    let first = {
        let publishing = publishing.clone();
        tokio::spawn(async move { publishing.sweep(ready()).await })
    };
    until_waiting(&publishing).await;
    sqlx::query("UPDATE builds.jobs SET publish_attempts = 2 WHERE id = $1::uuid")
        .bind(&held.job)
        .execute(&pool)
        .await
        .unwrap();
    publishing.fake.with(|store| {
        store.tamper = Some(sums);
        store.gate.notify_waiters();
    });
    assert_eq!(first.await.unwrap(), 1);
    assert_eq!(publishing.job(&held).await, ("uploading".to_owned(), None));
    assert!(publishing.fake.with(|store| store.deletes.is_empty()));
}

/// N-8: a job handed over before publishing was turned off fails with a fixed reason, instead
/// of waiting for a publisher that never comes.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_hub_with_publishing_off_fails_the_jobs_it_was_given(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let (bus, updates) = (Bus::new(16), Updates::new());
    let queue = publishing.queue(&bus, &updates);
    assert_eq!(builds::publishing_off(queue, ready()).await.unwrap(), 1);
    let (state, reason) = publishing.job(&held).await;
    assert_eq!(state, "failed");
    assert_eq!(
        reason.as_deref(),
        Some(
            "Publishing is off on this hub: no release bucket, or no usable updater key (the hub log says which)."
        )
    );
    assert_eq!(builds::publishing_off(queue, ready()).await.unwrap(), 0);
}

/// D-25: the operator cannot cancel a job the hub publishes; only a queued job is cancelled.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_job_the_hub_publishes_cannot_be_cancelled(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    let cancel = "mutation($id: ID!) { cancelBuildJob(id: $id) { state } }";
    let response = publishing
        .hub
        .post(
            Some(&publishing.operator),
            cancel,
            json!({ "id": held.job }),
        )
        .await;
    assert_eq!(error(&response).1, "CONFLICT", "{response}");
    assert_eq!(publishing.job(&held).await.0, "uploading");
}

/// The constraint `sql` breaks, if it fails.
async fn broken(pool: &PgPool, sql: &str, job: &str) -> Option<String> {
    let result = sqlx::query(sql).bind(job).execute(pool).await;
    let error = result.err()?;
    error
        .as_database_error()
        .and_then(|e| e.constraint())
        .map(str::to_owned)
}

#[sqlx::test(migrations = "./migrations")]
async fn d_25_the_database_keeps_the_publishing_rules(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, _) = uploaded_linux(&publishing, "0.1.12").await;
    publishing.release("0.1.13", "LINUX_X86_64").await;
    let queued: String =
        sqlx::query_scalar("SELECT id::text FROM builds.jobs WHERE state = 'queued'")
            .fetch_one(&pool)
            .await
            .unwrap();
    for (sql, job, constraint) in [
        (
            "UPDATE builds.jobs SET publish_attempts = 4 WHERE id = $1::uuid",
            &held.job,
            "jobs_publish_attempts_check",
        ),
        (
            "UPDATE builds.jobs SET publish_attempts = -1 WHERE id = $1::uuid",
            &held.job,
            "jobs_publish_attempts_check",
        ),
        (
            "UPDATE builds.jobs SET publish_held_until = now() WHERE id = $1::uuid",
            &queued,
            "jobs_publish_held_check",
        ),
        (
            "UPDATE builds.jobs SET publishing_since = now() WHERE id = $1::uuid",
            &queued,
            "jobs_publishing_check",
        ),
        (
            "UPDATE builds.jobs SET publishing_since = NULL, publish_held_until = NULL \
             WHERE id = $1::uuid",
            &held.job,
            "jobs_lease_check",
        ),
        (
            "UPDATE builds.feeds SET written = array['linux-x86_64', 'windows'] \
             WHERE job_id = $1::uuid",
            &held.job,
            "feeds_written_check",
        ),
    ] {
        assert_eq!(
            broken(&pool, sql, job).await.as_deref(),
            Some(constraint),
            "{sql}"
        );
    }
}
