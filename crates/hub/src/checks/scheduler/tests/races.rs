//! Writes that land while a probe runs: the attempt is evaluated on the state they left, not
//! on the state read at tick time (CHK-002, CHK-005).

use chrono::TimeDelta;

use super::support::{
    DOCKER_HOST, Listener, add, beats, database_now, on_loopback, rng, scheduler, stored,
};
use crate::{
    checks::{self, Beat, CheckState},
    events::{Bus, Signal},
    status::Status,
};

tick_test! {
    async fn rev_pause_during_probe_is_overwritten(pool: PgPool) {
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

    async fn rev_push_during_probe_is_lost(pool: PgPool) {
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
