//! Beats from any writer reach the status bookkeeping: pushes near a tick, a failed read,
//! devices seen first after the start and deleted devices (LOG-004, LOG-010).

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::sync::broadcast::Receiver;

use super::support::{
    DOCKER_HOST, Listener, add, beats, database_now, due_at, events_of, on_loopback, rng,
    scheduler, tcp,
};
use crate::{
    checks::{self, Beat, CheckState, Definition, PushToken, Recorded},
    events::{Bus, Signal},
    ids::{CheckId, DeviceId},
    status::Status,
};

fn device_statuses(receiver: &mut Receiver<Signal>) -> Vec<Status> {
    std::iter::from_fn(|| receiver.try_recv().ok())
        .filter_map(|signal| match signal {
            Signal::DeviceStatus { status, .. } => Some(status),
            _ => None,
        })
        .collect()
}

/// A PUSH check on `docker-host` that is not due for an hour.
async fn push_check(pool: &PgPool, now: DateTime<Utc>) -> (DeviceId, CheckId) {
    let device = on_loopback(pool, DOCKER_HOST).await;
    let push = Definition::Push(PushToken::from_random(&[5; 32]));
    let id = add(pool, device, push, 1).await;
    due_at(pool, id, now + TimeDelta::hours(1)).await;
    (device, id)
}

/// Records a push at `at`, as the push route does.
async fn push(pool: &PgPool, id: CheckId, at: DateTime<Utc>) {
    let beat = Beat {
        at,
        status: Status::Up,
        response_time_ms: None,
        message: None,
        state: CheckState {
            status: Status::Up,
            failures: 0,
            next_due_at: Some(at + TimeDelta::seconds(10)),
        },
        resolved_ip: None,
        cert_not_after: None,
    };
    let mut conn = pool.acquire().await.unwrap();
    let written = checks::record_beat(&mut conn, id, &beat).await.unwrap();
    assert_eq!(written, Recorded::Written);
}

async fn set_status_up(pool: &PgPool, id: CheckId) {
    sqlx::query("UPDATE checks.checks SET status = 'up' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

async fn run(pool: &PgPool, sql: &str) {
    sqlx::query(sql).execute(pool).await.unwrap();
}

tick_test! {
    async fn log_004_a_push_that_commits_after_a_tick_with_an_earlier_time_is_settled(pool: PgPool) {
        let now = database_now(&pool).await;
        let (_, id) = push_check(&pool, now).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);
        scheduler.tick(now, &mut rng()).await;

        push(&pool, id, now - TimeDelta::milliseconds(200)).await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;
        scheduler.tick(now + TimeDelta::seconds(2), &mut rng()).await;

        assert_eq!(device_statuses(&mut receiver), [Status::Up]);
    }

    async fn log_004_a_push_exactly_at_the_overlap_edge_is_settled(pool: PgPool) {
        let now = database_now(&pool).await;
        let (_, id) = push_check(&pool, now).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);
        scheduler.tick(now, &mut rng()).await;

        push(&pool, id, now - TimeDelta::seconds(5)).await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;

        assert_eq!(device_statuses(&mut receiver), [Status::Up]);
    }

    async fn log_004_a_failed_read_of_new_beats_reads_the_same_interval_again(pool: PgPool) {
        let now = database_now(&pool).await;
        let (_, id) = push_check(&pool, now).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);
        scheduler.tick(now, &mut rng()).await;
        push(&pool, id, now + TimeDelta::seconds(1)).await;

        run(&pool, "ALTER TABLE checks.checks RENAME COLUMN last_beat_at TO moved_away").await;
        scheduler.tick(now + TimeDelta::seconds(10), &mut rng()).await;
        run(&pool, "ALTER TABLE checks.checks RENAME COLUMN moved_away TO last_beat_at").await;
        assert_eq!(device_statuses(&mut receiver).len(), 0);
        scheduler.tick(now + TimeDelta::seconds(20), &mut rng()).await;

        assert_eq!(device_statuses(&mut receiver), [Status::Up]);
    }

    async fn log_004_a_push_on_a_device_already_up_gives_no_event_after_a_start(pool: PgPool) {
        let now = database_now(&pool).await;
        let (device, id) = push_check(&pool, now).await;
        set_status_up(&pool, id).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);
        scheduler.tick(now, &mut rng()).await;

        push(&pool, id, now + TimeDelta::seconds(1)).await;
        scheduler.tick(now + TimeDelta::seconds(2), &mut rng()).await;

        assert_eq!(device_statuses(&mut receiver).len(), 0);
        assert_eq!(events_of(&pool, device).await.len(), 0);
    }

    async fn log_004_a_device_added_after_the_start_gives_no_false_change(pool: PgPool) {
        let listener = Listener::start().await;
        let now = database_now(&pool).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);
        scheduler.tick(now, &mut rng()).await;
        let device = DeviceId::from(uuid::Uuid::now_v7());
        sqlx::query(
            "INSERT INTO devices.devices (id, site_id, name, host, kind, os, maintenance) \
             VALUES ($1, '0190a000-0000-7000-8001-000000000001', 'late-box', '127.0.0.1', \
                     'Test box', 'linux', false)",
        )
        .bind(device)
        .execute(&pool)
        .await
        .unwrap();
        let id = add(&pool, device, tcp(listener.port), 2).await;
        set_status_up(&pool, id).await;
        let at = now + TimeDelta::seconds(1);
        due_at(&pool, id, at).await;

        scheduler.tick(at, &mut rng()).await;
        scheduler.finish_running(at).await;

        assert_eq!(beats(&pool, id).await, ["up"]);
        assert_eq!(device_statuses(&mut receiver).len(), 0);
    }

    async fn log_004_a_deleted_device_is_forgotten(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));
        scheduler.tick(now, &mut rng()).await;
        assert!(scheduler.tracks(device));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while beats(&pool, id).await.is_empty() {
            assert!(tokio::time::Instant::now() < deadline, "no beat");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;

        run_delete(&pool, device).await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;

        assert!(!scheduler.tracks(device));
    }
}

async fn run_delete(pool: &PgPool, device: DeviceId) {
    sqlx::query("DELETE FROM devices.devices WHERE id = $1")
        .bind(device)
        .execute(pool)
        .await
        .unwrap();
}
