//! Writes that land while a probe runs: the attempt is evaluated on the state they left, not
//! on the state read at tick time (CHK-002, CHK-005).

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::task::JoinSet;

use super::support::{
    DOCKER_HOST, Listener, add, add_check, beats, database_now, device, due_at, new_check,
    on_loopback, rng, scheduler, stored, tcp, vault,
};
use crate::{
    checks::{
        self, Attempt, Beat, CheckChange, CheckState,
        probe::{Failure, ProbeResult},
    },
    events::{Bus, Signal},
    ids::CheckId,
    status::Status,
};

/// Records one failed attempt at `at` in its own transaction; returns whether it was written.
async fn failed_attempt(pool: PgPool, id: CheckId, at: DateTime<Utc>) -> bool {
    let result = ProbeResult::failed(&Failure::NoPush);
    let attempt = Attempt {
        result: &result,
        in_maintenance: false,
        at,
    };
    let mut tx = pool.begin().await.unwrap();
    let beat = checks::record_attempt(&mut tx, id, &attempt).await.unwrap();
    tx.commit().await.unwrap();
    beat.is_some()
}

tick_test! {
    async fn chk_002_concurrent_failed_attempts_count_every_written_failure(pool: PgPool) {
        for round in 0..30 {
            let new = new_check(device(DOCKER_HOST), &format!("F{round}"), tcp(22), 10);
            let id = add_check(&pool, new).await;
            let base = database_now(&pool).await - TimeDelta::seconds(30);
            let mut attempts = JoinSet::new();
            for i in 0..10 {
                let at = base + TimeDelta::milliseconds(i);
                attempts.spawn(failed_attempt(pool.clone(), id, at));
            }
            let all = tokio::time::timeout(Duration::from_secs(30), attempts.join_all()).await;
            let written = all.expect("the attempts finish").into_iter().filter(|w| *w).count();
            let failures = stored(&pool, id).await.state.failures;
            assert_eq!(usize::try_from(failures).unwrap(), written, "round {round}");
        }
    }

    async fn chk_002_a_settings_change_during_a_probe_applies_to_its_attempt(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        let change = CheckChange {
            retries: Some(0),
            ..CheckChange::default()
        };
        let mut conn = pool.acquire().await.unwrap();
        checks::update_check(&mut conn, &vault(), id, change).await.unwrap();
        scheduler.finish_running(now).await;

        assert_eq!(beats(&pool, id).await, ["down"]);
    }

    async fn chk_005_a_pause_during_a_probe_wins(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        sqlx::query("UPDATE checks.checks SET next_due_at = $2 WHERE id = $1")
            .bind(id)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        let mut conn = pool.acquire().await.unwrap();
        checks::set_paused(&mut conn, id, true).await.unwrap();
        scheduler.finish_running(now).await;

        assert_eq!(beats(&pool, id).await.len(), 0);
        let signals: Vec<Signal> = std::iter::from_fn(|| receiver.try_recv().ok()).collect();
        assert!(!signals.iter().any(|s| matches!(s, Signal::Heartbeat { .. })), "{signals:?}");
        let check = stored(&pool, id).await;
        assert!(check.paused);
        assert_eq!(check.state.failures, 0);
    }

    async fn chk_002_an_attempt_is_evaluated_on_a_push_made_during_its_probe(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, listener.silent_tls(), 1).await;
        let now = database_now(&pool).await;
        sqlx::query(
            "UPDATE checks.checks SET status = 'up', failures = 1, next_due_at = $2 WHERE id = $1",
        )
        .bind(id)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        let pushed_at = now - TimeDelta::seconds(1);
        let push = Beat {
            at: pushed_at,
            status: Status::Up,
            response_time_ms: None,
            message: None,
            state: CheckState {
                status: Status::Up,
                failures: 0,
                next_due_at: Some(pushed_at + TimeDelta::seconds(10)),
            },
            resolved_ip: None,
            cert_not_after: None,
        };
        let mut conn = pool.acquire().await.unwrap();
        let written = checks::record_beat(&mut conn, id, &push).await.unwrap();
        assert_eq!(written, checks::Recorded::Written);
        scheduler.finish_running(now).await;

        assert_eq!(beats(&pool, id).await, ["up", "unknown"]);
        let state = stored(&pool, id).await.state;
        assert_eq!((state.status, state.failures), (Status::Up, 1));
    }
}
