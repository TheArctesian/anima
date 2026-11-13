//! A push and a scheduled attempt that run at the same time on one check count every written
//! failure: both evaluate on the state the row lock leaves them (CHK-002, CHK-003). An attempt
//! queues its notifications only with a written beat, and in the beat's own transaction
//! (D-16, NOT-001).

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

use super::support::{
    DOCKER_HOST, add, add_check, database_now, device, new_check, notify_on, queued, stored, tcp,
};
use crate::{
    clock::Clock,
    fleet::{
        checks::{
            self, Attempt, Definition, PushToken,
            probe::{Failure, ProbeResult},
            push::push_router_within,
            push_router,
        },
        devices,
        events::Bus,
    },
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
    let owner = checks::get(&pool, id).await.unwrap().device;
    let device = devices::get(&pool, owner).await.unwrap();
    let attempt = Attempt {
        result: &result,
        in_maintenance: false,
        at,
        domain: None,
        device: &device,
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

/// Records one failed attempt at `at` on a connection with no transaction, so the attempt's
/// own transaction is the one that commits; returns whether the beat was written.
async fn attempt_alone(pool: PgPool, id: CheckId, at: DateTime<Utc>) -> bool {
    let result = ProbeResult::failed(&Failure::NoPush);
    let device = devices::get(&pool, device(DOCKER_HOST)).await.unwrap();
    let attempt = Attempt {
        result: &result,
        in_maintenance: false,
        at,
        domain: None,
        device: &device,
    };
    let mut conn = pool.acquire().await.unwrap();
    checks::record_attempt(&mut conn, id, &attempt)
        .await
        .unwrap()
        .is_some()
}

/// How many notification state rows `check` has.
async fn states(pool: &PgPool, id: CheckId) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM notifications.check_state WHERE check_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Waits until some statement of this database waits for a row lock.
async fn until_a_statement_waits(pool: &PgPool) {
    for _ in 0..500 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_catalog.pg_stat_activity \
             WHERE datname = current_database() AND wait_event_type = 'Lock'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no statement waited for the lock");
}

tick_test! {
    async fn d_16_a_skipped_attempt_queues_nothing_and_keeps_no_state(pool: PgPool) {
        let at = database_now(&pool).await;
        let paused = add_check(&pool, new_check(device(DOCKER_HOST), "Paused", tcp(1), 0)).await;
        let stale = add_check(&pool, new_check(device(DOCKER_HOST), "Stale", tcp(2), 0)).await;
        for id in [paused, stale] {
            notify_on(&pool, id).await;
        }
        sqlx::query("UPDATE checks.checks SET paused = true WHERE id = $1")
            .bind(paused)
            .execute(&pool)
            .await
            .unwrap();
        // A newer beat landed while the attempt ran.
        sqlx::query("UPDATE checks.checks SET last_beat_at = $2 WHERE id = $1")
            .bind(stale)
            .bind(at + TimeDelta::seconds(10))
            .execute(&pool)
            .await
            .unwrap();

        for id in [paused, stale] {
            assert!(!attempt_alone(pool.clone(), id, at).await, "the beat is skipped");
            assert_eq!(queued(&pool, id).await, Vec::<String>::new());
            assert_eq!(states(&pool, id).await, 0);
        }
    }

    async fn d_16_the_beat_commits_only_with_its_notifications(pool: PgPool) {
        let id = add(&pool, device(DOCKER_HOST), tcp(1), 0).await;
        notify_on(&pool, id).await;
        sqlx::query("INSERT INTO notifications.check_state (check_id) VALUES ($1)")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        let mut holder = pool.begin().await.unwrap();
        sqlx::query("SELECT 1 FROM notifications.check_state WHERE check_id = $1 FOR UPDATE")
            .bind(id)
            .execute(&mut *holder)
            .await
            .unwrap();
        let at = database_now(&pool).await;
        let writer = tokio::spawn(attempt_alone(pool.clone(), id, at));

        until_a_statement_waits(&pool).await;
        // The check row, not the heartbeats: a read of the partitioned table would wait for
        // the day partition the attempt may have created in its open transaction.
        let visible: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT last_beat_at FROM checks.checks WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        holder.rollback().await.unwrap();

        assert_eq!(visible, None, "the beat was visible before its notifications were queued");
        let written = tokio::time::timeout(Duration::from_secs(10), writer).await;
        assert!(written.expect("the attempt finishes").unwrap(), "the beat is written");
        assert_eq!(queued(&pool, id).await, ["down"]);
    }
}
