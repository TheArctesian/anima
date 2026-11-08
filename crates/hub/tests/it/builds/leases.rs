//! Claims are atomic and an ended lease requeues the job, failing it the third time
//! (BLD-004 to BLD-006, BLD-029, BLD-082, D-2).

use anima_hub::{
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

#[sqlx::test(migrations = "./migrations")]
async fn bld_029_concurrent_claims_of_one_builder_take_one_lease(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    for patch in 1..=6 {
        create_job(&hub, &operator, &format!("0.1.{patch}")).await;
    }
    let builder = enrol(&hub, &operator, "pop-os").await;
    let claims = (0..16).map(|_| claim(&hub, &builder.token));
    let responses = async_graphql::futures_util::future::join_all(claims).await;
    let claimed = responses
        .into_iter()
        .filter(|response| !data(response.clone())["claimBuildJob"].is_null())
        .count();
    assert_eq!(claimed, 1);
    let leased: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM builds.jobs WHERE builder_id IS NOT NULL AND state = 'claimed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leased, 1);
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
