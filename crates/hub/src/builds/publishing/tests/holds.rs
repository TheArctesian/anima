//! A try that no longer holds its job leaves the job's row unlocked when `finish` returns, so
//! the same sweep's next take (`FOR UPDATE SKIP LOCKED`) can take it again (D-25); so does a
//! refused request for upload URLs (D-43).
//!
//! The lock is checked from another thread with its own runtime and connection while this
//! test's thread is blocked. A transaction that was dropped instead of ended rolls back only
//! once a task of this test's single-thread runtime runs, so it still holds the lock during the
//! check: the result does not depend on load.

use chrono::{DateTime, TimeDelta, Utc};
use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::{
    Bucket, Grant,
    run::{Outcome, Reach, Tried},
    tries, upload_urls,
};
use crate::{
    builds::{
        BuildQueue, Error, Updates,
        model::UploadedFile,
        tests::{
            BUILDER, builder,
            holds::{queued_job, row_locked},
        },
    },
    clock::FixedClock,
    config::Releases,
    events::Bus,
};

const OPERATOR: &str = "0192f5a0-7c3e-7a52-9d1b-000000000001";
const RELEASE: &str = "0192f5a0-7c3e-7a52-9d1b-0000000000a1";
const JOB: &str = "0192f5a0-7c3e-7a52-9d1b-0000000000b1";

fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_900_000_000, 0).unwrap()
}

/// A Linux job the publisher holds as try 1 until 10 minutes after `now()`, its tries at
/// `attempts`.
async fn publishing_job(pool: &PgPool, attempts: i32) {
    sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::uuid, 'operator', '$argon2id$x', $2)",
    )
    .bind(OPERATOR)
    .bind(now())
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into builds.releases (id, version, commit_sha, created_by, created_by_name, \
                created_at) \
         values ($1::uuid, '0.1.12', repeat('a', 40), $2::uuid, 'operator', $3)",
    )
    .bind(RELEASE)
    .bind(OPERATOR)
    .bind(now())
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into builds.jobs (id, release_id, target, version, state, created_at, \
                queued_at, uploading_at, state_changed_at, publishing_since, \
                publish_held_until, publish_attempts) \
         values ($1::uuid, $2::uuid, 'linux-x86_64', '0.1.12', 'uploading', $3, $3, $3, $3, \
                 $3, $4, $5)",
    )
    .bind(JOB)
    .bind(RELEASE)
    .bind(now())
    .bind(now() + TimeDelta::minutes(10))
    .bind(attempts)
    .execute(pool)
    .await
    .unwrap();
}

/// A bucket on a closed loopback port: a try that leaves its job reaches no bucket.
fn bucket(tokens: &tempfile::TempDir) -> Bucket {
    std::fs::write(tokens.path().join("id"), "AKIDTEST\n").unwrap();
    std::fs::write(tokens.path().join("secret"), "test-secret").unwrap();
    Bucket::open(
        &Releases {
            endpoint: "http://127.0.0.1:9".to_owned(),
            bucket: "anima-releases".to_owned(),
            public_base: "http://127.0.0.1:9".to_owned(),
            access_key_id_file: tokens.path().join("id"),
            secret_access_key_file: tokens.path().join("secret"),
            updater_public_key: None,
            updater_key_file: tokens.path().join("updater_key"),
            updater_key_password_file: tokens.path().join("updater_key_password"),
        },
        super::updater_key(),
    )
    .unwrap()
}

/// Ends try 1 of the job with `outcome`.
async fn finish(pool: &PgPool, outcome: Outcome) {
    let tokens = tempfile::tempdir().unwrap();
    let bucket = bucket(&tokens);
    let (bus, updates, http) = (Bus::new(16), Updates::new(), Client::new());
    let build_queue = BuildQueue {
        db: pool,
        bus: &bus,
        updates: &updates,
    };
    let clock = FixedClock(now() + TimeDelta::minutes(11));
    let reach = Reach {
        bucket: &bucket,
        http: &http,
        clock: &clock,
    };
    let tried = Tried {
        job: Uuid::parse_str(JOB).unwrap().into(),
        attempt: 1,
    };
    tries::finish(build_queue, reach, tried, outcome)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_taken_over_try_leaves_the_job_unlocked(pool: PgPool) {
    publishing_job(&pool, 1).await;
    finish(&pool, Outcome::TakenOver).await;
    assert!(!row_locked((*pool.connect_options()).clone(), JOB));
}

#[sqlx::test(migrations = "./migrations")]
async fn d_25_a_try_a_later_one_replaced_leaves_the_job_unlocked(pool: PgPool) {
    publishing_job(&pool, 2).await;
    finish(&pool, Outcome::Unavailable("bucket read")).await;
    assert!(!row_locked((*pool.connect_options()).clone(), JOB));
}

#[sqlx::test(migrations = "./migrations")]
async fn d_43_a_refused_request_for_upload_urls_leaves_the_job_unlocked(pool: PgPool) {
    queued_job(&pool, JOB).await;
    let tokens = tempfile::tempdir().unwrap();
    let bucket = bucket(&tokens);
    let files = [UploadedFile {
        name: "SHA256SUMS".to_owned(),
        sha256: "0".repeat(64),
        size: 1,
    }];
    let grant = Grant {
        builder: &builder(BUILDER),
        job: JOB.parse().unwrap(),
        files: &files,
    };
    let refused = upload_urls(&pool, &bucket, grant, now()).await;
    assert!(matches!(refused, Err(Error::Forbidden(_))));
    assert!(!row_locked((*pool.connect_options()).clone(), JOB));
}
