//! MET-012, MET-013, K8 Section 3: the device row orders an enrolment against a revoke and
//! against a second code. A revoke that commits after the code was redeemed stops the enrolment;
//! one that starts while the enrolment is storing its token waits, then deletes the token; a
//! second code made while the first is being stored waits, then voids it. A stored name the agent
//! would not keep enrols nothing.

use std::time::Duration;

use agent_core::wire::{AgentToken, EnrolmentCode};
use sqlx::PgPool;
use tokio::task::JoinHandle;

use super::{database_now, enrolled_pve, pve};
use crate::{
    events::Bus,
    metrics::ingest::{
        credential::authenticate,
        enrolment::{enrol, redeem},
        make_code, revoke,
    },
};

/// Whether `task` came to wait before it finished, seen as `count` sessions of this test's
/// database waiting for a lock. Read from `pg_stat_activity`, so the answer does not depend on
/// how fast either side runs.
pub(super) async fn waits_for_locks<T>(pool: &PgPool, task: &JoinHandle<T>, count: i64) -> bool {
    for _ in 0..2000 {
        if task.is_finished() {
            return false;
        }
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_catalog.pg_stat_activity \
             WHERE datname = pg_catalog.current_database() AND wait_event_type = 'Lock'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting >= count {
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
    enrolled_pve(&pool, now, 1).await;
    let code = EnrolmentCode::from_random([5; 16]);
    let presented = EnrolmentCode::parse(code.expose()).unwrap();
    let mut conn = pool.acquire().await.unwrap();
    make_code(&mut conn, pve(), code, now).await.unwrap();
    drop(conn);
    let redeemed = redeem(&pool, &presented, now).await.unwrap().unwrap();
    // Holds the agent row, so the enrolment stops after it took the device row.
    let mut holding = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM metrics.agents WHERE device_id = $1 FOR UPDATE")
        .bind(pve())
        .execute(&mut *holding)
        .await
        .unwrap();

    let token = AgentToken::from_random([7; 32]);
    let agent = AgentToken::parse(token.expose()).unwrap();
    let enrolling = tokio::spawn({
        let pool = pool.clone();
        async move {
            enrol(&pool, &Bus::new(4), (redeemed, token), now)
                .await
                .unwrap()
        }
    });
    assert!(
        waits_for_locks(&pool, &enrolling, 1).await,
        "the enrolment did not reach the agent row"
    );
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
        waits_for_locks(&pool, &revoking, 2).await,
        "the revoke ran beside the enrolment"
    );
    holding.commit().await.unwrap();
    assert!(enrolling.await.unwrap().is_some(), "the enrolment finished");
    assert!(
        revoking.await.unwrap(),
        "the revoke missed the enrolled token"
    );
    assert_eq!(agents(&pool).await, 0);
    assert_eq!(authenticate(&pool, &agent, now).await.unwrap(), None);
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
        waits_for_locks(&pool, &making, 1).await,
        "the second code was made beside the first"
    );
    storing.commit().await.unwrap();
    making.await.unwrap();

    assert!(redeem(&pool, &first_text, now).await.unwrap().is_none());
    assert!(redeem(&pool, &second_text, now).await.unwrap().is_some());
}

/// T8a N2: a stored name older than the device rule, which the agent would throw away, stores
/// no token and leaves the metrics source as it was.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_012_a_stored_name_the_agent_would_not_keep_enrols_nothing(pool: PgPool) {
    let now = database_now(&pool).await;
    for (seed, name) in [(5, "x".repeat(65)), (6, "pve\u{1b}[2K-01".to_owned())] {
        sqlx::query("UPDATE devices.devices SET name = $2 WHERE id = $1")
            .bind(pve())
            .bind(&name)
            .execute(&pool)
            .await
            .unwrap();
        let code = EnrolmentCode::from_random([seed; 16]);
        let presented = EnrolmentCode::parse(code.expose()).unwrap();
        let mut conn = pool.acquire().await.unwrap();
        make_code(&mut conn, pve(), code, now).await.unwrap();
        drop(conn);
        let redeemed = redeem(&pool, &presented, now).await.unwrap().unwrap();
        let token = AgentToken::from_random([seed; 32]);
        let enrolled = enrol(&pool, &Bus::new(4), (redeemed, token), now).await;
        assert!(enrolled.unwrap().is_none(), "{name:?}");
        assert_eq!(agents(&pool).await, 0, "{name:?}");
        let source: String =
            sqlx::query_scalar("SELECT metrics_source FROM devices.devices WHERE id = $1")
                .bind(pve())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(source, "none", "{name:?}");
    }
}
