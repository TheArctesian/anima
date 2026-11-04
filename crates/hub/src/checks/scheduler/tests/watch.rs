//! What a tick tells the rest of the hub: signals after commit, status-change events and
//! the watchdog (CHK-010, LOG-004, LOG-010).

use chrono::TimeDelta;
use tokio::sync::broadcast::Receiver;

use super::support::{
    DOCKER_HOST, Listener, add, database_now, due_at, events_of, on_loopback, rng, scheduler,
    settle_briefly, tcp,
};
use crate::{
    events::{Bus, Signal},
    status::Status,
};

fn drain(receiver: &mut Receiver<Signal>) -> Vec<Signal> {
    std::iter::from_fn(|| receiver.try_recv().ok()).collect()
}

fn device_statuses(signals: &[Signal]) -> Vec<Status> {
    signals
        .iter()
        .filter_map(|signal| match signal {
            Signal::DeviceStatus { status, .. } => Some(*status),
            _ => None,
        })
        .collect()
}

tick_test! {
    async fn log_010_the_heartbeat_signal_follows_the_commit(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let bus = Bus::new(16);
        let mut receiver = bus.subscribe();
        let reader = pool.clone();
        let seen = tokio::spawn(async move {
            loop {
                if let Signal::Heartbeat { check, beat, .. } = receiver.recv().await.unwrap() {
                    let stored: i64 = sqlx::query_scalar(
                        "SELECT count(*) FROM checks.heartbeats WHERE check_id = $1",
                    )
                    .bind(check)
                    .fetch_one(&reader)
                    .await
                    .unwrap();
                    return (check, beat.status, stored);
                }
            }
        });
        let mut scheduler = scheduler(&pool, &bus);

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        assert_eq!(seen.await.unwrap(), (id, Status::Up, 1));
    }

    async fn log_004_a_status_change_gives_one_event_and_one_signal(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 0).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;
        let later = now + TimeDelta::seconds(10);
        scheduler.tick(later, &mut rng()).await;
        scheduler.finish_running(later).await;

        let signals = drain(&mut receiver);
        assert_eq!(device_statuses(&signals), [Status::Up]);
        assert!(signals.contains(&Signal::DeviceStatus { device, status: Status::Up, at: now }));
        let up = ("info".to_owned(), "check".to_owned(), "docker-host is Up".to_owned());
        assert_eq!(events_of(&pool, device).await, std::slice::from_ref(&up));

        sqlx::query("UPDATE checks.checks SET config = jsonb_set(config, '{port}', to_jsonb($2)) WHERE id = $1")
            .bind(id)
            .bind(i32::from(super::support::closed_port()))
            .execute(&pool)
            .await
            .unwrap();
        let down_at = now + TimeDelta::seconds(20);
        scheduler.tick(down_at, &mut rng()).await;
        scheduler.finish_running(down_at).await;

        assert_eq!(device_statuses(&drain(&mut receiver)), [Status::Down]);
        let down = ("error".to_owned(), "check".to_owned(), "docker-host is Down".to_owned());
        assert_eq!(events_of(&pool, device).await, [up, down]);
    }

    async fn chk_010_a_late_check_gives_exactly_one_error_event(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let late = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, late, now - TimeDelta::seconds(25)).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        scheduler.tick(now, &mut rng()).await;
        settle_briefly().await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;
        scheduler.finish_running(now).await;

        let message = "Check Service port on docker-host is late by more than 2 intervals";
        let expected = ("error".to_owned(), "check".to_owned(), message.to_owned());
        assert_eq!(events_of(&pool, device).await, [expected]);
    }

    async fn chk_010_a_check_within_2_intervals_is_not_late(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now - TimeDelta::seconds(20)).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        let events = events_of(&pool, device).await;
        assert!(events.iter().all(|(severity, ..)| severity != "error"), "{events:?}");
    }
}
