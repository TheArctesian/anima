//! A push and a scheduled attempt that run at the same time on one check count every written
//! failure: both evaluate on the state the row lock leaves them (CHK-002, CHK-003).

use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::task::JoinSet;
use tower::ServiceExt as _;

use super::support::{DOCKER_HOST, add_check, database_now, device, new_check, stored};
use crate::{
    checks::{
        self, Attempt, Definition, PushToken,
        probe::{Failure, ProbeResult},
        push::push_router_within,
        push_router,
    },
    clock::Clock,
    events::Bus,
    ids::CheckId,
};

/// A clock that reports one millisecond later on every call.
struct Stepping(AtomicI64, DateTime<Utc>);

impl Clock for Stepping {
    fn now(&self) -> DateTime<Utc> {
        self.1 + TimeDelta::milliseconds(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

async fn failed_attempt(pool: PgPool, id: CheckId, at: DateTime<Utc>) {
    let result = ProbeResult::failed(&Failure::NoPush);
    let attempt = Attempt {
        result: &result,
        in_maintenance: false,
        at,
        domain: None,
    };
    let mut tx = pool.begin().await.unwrap();
    checks::store::record_attempt(&mut tx, id, &attempt)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn failed_push(router: axum::Router, path: String) {
    let request = Request::builder()
        .uri(format!("{path}?status=down"))
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

tick_test! {
    async fn chk_002_concurrent_pushes_and_attempts_count_every_written_failure(pool: PgPool) {
        for round in 0..20_u8 {
            let token = PushToken::from_random(&[round; 32]);
            let path = format!("/push/{}", token.expose());
            let new = new_check(device(DOCKER_HOST), &format!("P{round}"), Definition::Push(token), 10);
            let id = add_check(&pool, new).await;
            let base = database_now(&pool).await - TimeDelta::seconds(30);
            let clock: Arc<dyn Clock> = Arc::new(Stepping(AtomicI64::new(100), base));
            let router = push_router(pool.clone(), Bus::new(16), clock);
            let mut running = JoinSet::new();
            for i in 0..6 {
                running.spawn(failed_push(router.clone(), path.clone()));
                let at = base + TimeDelta::milliseconds(i);
                running.spawn(failed_attempt(pool.clone(), id, at));
            }
            let all = tokio::time::timeout(Duration::from_secs(30), running.join_all()).await;
            all.expect("the requests finish");
            let written: i64 = sqlx::query_scalar("SELECT count(*) FROM checks.heartbeats WHERE check_id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
            let failures = stored(&pool, id).await.state.failures;
            assert!(written > 0, "round {round}");
            assert_eq!(i64::from(failures), written, "round {round}");
        }
    }

    async fn net_002_a_push_held_past_its_answer_time_gets_503_and_records_nothing(pool: PgPool) {
        let token = PushToken::from_random(&[9; 32]);
        let path = format!("/push/{}", token.expose());
        let new = new_check(device(DOCKER_HOST), "Held", Definition::Push(token), 10);
        let id = add_check(&pool, new).await;
        let clock: Arc<dyn Clock> = Arc::new(Stepping(AtomicI64::new(0), database_now(&pool).await));
        let router = push_router_within(pool.clone(), Bus::new(16), clock, Duration::from_millis(200));
        let mut holder = pool.begin().await.unwrap();
        sqlx::query("SELECT 1 FROM checks.checks WHERE id = $1 FOR UPDATE")
            .bind(id)
            .execute(&mut *holder)
            .await
            .unwrap();

        let request = Request::builder()
            .uri(format!("{path}?status=down"))
            .body(Body::empty())
            .unwrap();
        let response = tokio::time::timeout(Duration::from_secs(10), router.oneshot(request))
            .await
            .expect("the push answers by its answer time")
            .unwrap();
        holder.rollback().await.unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let written: i64 = sqlx::query_scalar("SELECT count(*) FROM checks.heartbeats WHERE check_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(written, 0);
    }
}
