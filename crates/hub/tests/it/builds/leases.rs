//! Claims are atomic and an ended lease requeues the job, failing it the third time
//! (BLD-004 to BLD-006, BLD-029, BLD-082, D-2).

use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use anima_hub::fleet::{
    builds::{self, BuildQueue, Updates},
    events::Bus,
};
use serde_json::Value;
use sqlx::PgPool;

use super::harness::{at, claim, create_job, data, enrol, hub_at, operator};
use crate::hub::{Hub, start};

#[sqlx::test(migrations = "./migrations")]
async fn bld_004_concurrent_claims_never_share_a_job(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let mut jobs = vec![
        create_job(&hub, &operator, "0.1.1").await,
        create_job(&hub, &operator, "0.1.2").await,
    ];
    let mut builders = Vec::new();
    for name in ["one", "two", "three"] {
        builders.push(enrol(&hub, &operator, name).await);
    }
    let (a, b, c) = tokio::join!(
        claim(&hub, &builders[0].token),
        claim(&hub, &builders[1].token),
        claim(&hub, &builders[2].token),
    );
    let mut claimed: Vec<String> = [a, b, c]
        .into_iter()
        .filter_map(|response| {
            let id = &data(response)["claimBuildJob"]["id"];
            id.as_str().map(str::to_owned)
        })
        .collect();
    claimed.sort();
    jobs.sort();
    assert_eq!(claimed, jobs);
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_029_a_builder_holding_a_lease_claims_nothing_more(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    create_job(&hub, &operator, "0.1.1").await;
    create_job(&hub, &operator, "0.1.2").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    let first = data(claim(&hub, &builder.token).await);
    assert_eq!(first["claimBuildJob"]["state"], "CLAIMED");
    assert_eq!(first["claimBuildJob"]["builderName"], "pop-os");
    let second = data(claim(&hub, &builder.token).await);
    assert_eq!(second["claimBuildJob"], Value::Null);
}

/// How many sessions of this database wait for a row lock.
async fn lock_waiters(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_stat_activity \
         WHERE datname = current_database() AND wait_event_type = 'Lock'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_029_concurrent_claims_of_one_builder_take_one_lease(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    for patch in 1..=2 {
        create_job(&hub, &operator, &format!("0.1.{patch}")).await;
    }
    enrol(&hub, &operator, "pop-os").await;
    let builder = builds::builders(&pool).await.unwrap().remove(0);
    let (bus, updates) = (Bus::new(4), Updates::new());
    let queue = BuildQueue {
        db: &pool,
        bus: &bus,
        updates: &updates,
    };
    // Another claim of this builder is under way: it holds the builder row as a claim does.
    // The two claims below must wait for it, so neither can read the builder's leases before
    // the other's lease is written (a job update's foreign key check does not wait for it).
    let mut first = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM builds.builders WHERE id = $1 FOR NO KEY UPDATE")
        .bind(builder.id)
        .execute(&mut *first)
        .await
        .unwrap();
    let finished = AtomicBool::new(false);
    let claims = async {
        let both = tokio::join!(
            builds::claim(queue, &builder, at(0)),
            builds::claim(queue, &builder, at(0)),
        );
        finished.store(true, Ordering::SeqCst);
        both
    };
    let gate = async {
        for _ in 0..500 {
            if finished.load(Ordering::SeqCst) {
                return false;
            }
            if lock_waiters(&pool).await == 2 {
                first.rollback().await.unwrap();
                return true;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        false
    };
    let ((a, b), waited) = tokio::join!(claims, gate);
    assert!(
        waited,
        "the claims did not wait for the builder's other claim"
    );
    let claimed = [a.unwrap(), b.unwrap()].into_iter().flatten().count();
    assert_eq!(claimed, 1);
    let leased: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM builds.jobs WHERE builder_id IS NOT NULL AND state = 'claimed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leased, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_3_a_claim_skips_a_job_another_transaction_holds_without_waiting(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let held = create_job(&hub, &operator, "0.1.1").await;
    let free = create_job(&hub, &operator, "0.1.2").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    let mut holder = pool.begin().await.unwrap();
    let locked: String = sqlx::query_scalar(
        "SELECT id::text FROM builds.jobs ORDER BY created_at, id LIMIT 1 FOR UPDATE",
    )
    .fetch_one(&mut *holder)
    .await
    .unwrap();
    assert_eq!(locked, held);
    let claimed = tokio::time::timeout(Duration::from_secs(5), claim(&hub, &builder.token))
        .await
        .expect("the claim waited for the held job");
    assert_eq!(data(claimed)["claimBuildJob"]["id"], free.as_str());
    holder.rollback().await.unwrap();
}

async fn failure_events(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT message FROM events.event_log WHERE source = 'hub' AND severity = 'error'",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_006_an_ended_lease_requeues_the_job_and_fails_it_the_third_time(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let job = create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    for (seconds, attempts) in [(0, 0), (90, 1), (180, 2)] {
        let claimed = data(claim(&hub_at(&pool, seconds), &builder.token).await);
        assert_eq!(claimed["claimBuildJob"]["id"], job.as_str());
        assert_eq!(claimed["claimBuildJob"]["attempts"], attempts);
    }
    assert_eq!(failure_events(&pool).await, Vec::<String>::new());
    let after = data(claim(&hub_at(&pool, 270), &builder.token).await);
    assert_eq!(after["claimBuildJob"], Value::Null);

    let job: (String, i32, Option<String>) =
        sqlx::query_as("SELECT state, attempts, failure_reason FROM builds.jobs")
            .fetch_one(&pool)
            .await
            .unwrap();
    let reason = "builder stopped 3 times".to_owned();
    assert_eq!(job, ("failed".to_owned(), 3, Some(reason)));
    let events = failure_events(&pool).await;
    assert_eq!(events.len(), 1, "{events:?}");
    assert!(events[0].contains("0.1.12"), "{events:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_2_the_upkeep_sweep_ends_leases_that_ran_out(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    data(claim(&hub, &builder.token).await);
    let (bus, updates) = (Bus::new(4), Updates::new());
    let queue = BuildQueue {
        db: &pool,
        bus: &bus,
        updates: &updates,
    };
    assert_eq!(builds::expire_leases(queue, at(89)).await.unwrap(), 0);
    assert_eq!(builds::expire_leases(queue, at(90)).await.unwrap(), 1);
    let job: (String, i32) = sqlx::query_as("SELECT state, attempts FROM builds.jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(job, ("queued".to_owned(), 1));
}
