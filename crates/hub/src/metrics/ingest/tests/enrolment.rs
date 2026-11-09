//! MET-012, MET-013, K8 Section 3: the device row orders an enrolment against a revoke and
//! against a second code. A revoke that commits after the code was redeemed stops the enrolment;
//! one that starts while the enrolment is storing its token waits, then deletes the token; a
//! second code made while the first is being stored waits, then voids it.

use std::time::Duration;

use agent_core::wire::{AgentToken, EnrolmentCode};
use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::task::JoinHandle;

use super::{database_now, pve};
use crate::{
    devices,
    events::Bus,
    metrics::ingest::{
        credential::authenticate,
        enrolment::{enrol, redeem},
        make_code, revoke, store,
    },
};

/// Whether `task` came to wait for a row lock in this test's database before it finished. Read
/// from `pg_stat_activity`, so the answer does not depend on how fast either side runs.
async fn waits_for_a_lock<T>(pool: &PgPool, task: &JoinHandle<T>) -> bool {
    for _ in 0..2000 {
        if task.is_finished() {
            return false;
        }
        let waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_stat_activity \
             WHERE datname = pg_catalog.current_database() AND wait_event_type = 'Lock')",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the task neither finished nor waited for a lock");
}

async fn agents(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM metrics.agents")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_013_a_revoke_after_the_code_was_redeemed_stops_the_enrolment(pool: PgPool) {
    let now = database_now(&pool).await;
    let code = EnrolmentCode::from_random([5; 16]);
    let presented = EnrolmentCode::parse(code.expose()).unwrap();
    let mut conn = pool.acquire().await.unwrap();
    make_code(&mut conn, pve(), code, now).await.unwrap();
    drop(conn);
    let redeemed = redeem(&pool, &presented, now).await.unwrap().unwrap();

    let mut revoking = pool.begin().await.unwrap();
    assert!(!revoke(&mut revoking, pve()).await.unwrap());
    revoking.commit().await.unwrap();

    let token = AgentToken::from_random([7; 32]);
    let agent = AgentToken::parse(token.expose()).unwrap();
    let enrolled = enrol(&pool, &Bus::new(4), (redeemed, token), now).await;
    assert!(enrolled.unwrap().is_none());
    assert_eq!(authenticate(&pool, &agent, now).await.unwrap(), None);
    let source: String =
        sqlx::query_scalar("SELECT metrics_source FROM devices.devices WHERE id = $1")
            .bind(pve())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(source, "none");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_013_a_revoke_waits_for_an_enrolment_in_flight_then_deletes_its_token(pool: PgPool) {
    let now = database_now(&pool).await;
    // An enrolment past its code check: the device row taken and its token row written.
    let mut enrolling = pool.begin().await.unwrap();
    devices::use_agent_metrics(&mut enrolling, pve())
        .await
        .unwrap();
    let token = AgentToken::from_random([7; 32]);
    let expiry = now + TimeDelta::days(365);
    store::replace_token(&mut enrolling, pve(), token.hash(), expiry)
        .await
        .unwrap();

    let revoking = tokio::spawn({
        let pool = pool.clone();
        async move {
            let mut tx = pool.begin().await.unwrap();
            let had_token = revoke(&mut tx, pve()).await.unwrap();
            tx.commit().await.unwrap();
            had_token
        }
    });
    assert!(
        waits_for_a_lock(&pool, &revoking).await,
        "the revoke ran beside the enrolment"
    );
    enrolling.commit().await.unwrap();
    assert!(
        revoking.await.unwrap(),
        "the revoke missed the enrolled token"
    );
    assert_eq!(agents(&pool).await, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_012_a_second_code_made_while_the_first_is_stored_waits_then_voids_it(pool: PgPool) {
    let now = database_now(&pool).await;
    let first = EnrolmentCode::from_random([5; 16]);
    let first_text = EnrolmentCode::parse(first.expose()).unwrap();
    let mut storing = pool.begin().await.unwrap();
    make_code(&mut storing, pve(), first, now).await.unwrap();

    let second = EnrolmentCode::from_random([6; 16]);
    let second_text = EnrolmentCode::parse(second.expose()).unwrap();
    let making = tokio::spawn({
        let pool = pool.clone();
        async move {
            let mut tx = pool.begin().await.unwrap();
            make_code(&mut tx, pve(), second, now).await.unwrap();
            tx.commit().await.unwrap();
        }
    });
    assert!(
        waits_for_a_lock(&pool, &making).await,
        "the second code was made beside the first"
    );
    storing.commit().await.unwrap();
    making.await.unwrap();

    assert!(redeem(&pool, &first_text, now).await.unwrap().is_none());
    assert!(redeem(&pool, &second_text, now).await.unwrap().is_some());
}
