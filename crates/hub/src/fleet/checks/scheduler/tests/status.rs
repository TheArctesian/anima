//! What a tick tells live views about devices: heartbeat signals after commit, and one
//! stored event plus one signal per status change, from the scheduler's beats or the push
//! route's (LOG-004, LOG-010). What it tells the operator's channels: one notification per
//! selected channel, queued in the beat's transaction, none under maintenance (NOT-001, D-16,
//! Section 6.5).

use chrono::TimeDelta;
use tokio::sync::broadcast::Receiver;

use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, JELLYFIN, Listener, add, allow_event_writes, closed_port, database_now, due_at,
    events_of, fail_event_writes, notify_on, on_loopback, queued, rng, scheduler, tcp,
};
use crate::{
    fleet::{
        checks::{
            self, Attempt, Beat, CheckState, Definition, PushToken,
            probe::{Failure, ProbeResult},
        },
        devices,
        events::{Bus, Signal},
    },
    ids::CheckId,
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

fn event(severity: &str, message: &str) -> (String, String, String) {
    (severity.to_owned(), "check".to_owned(), message.to_owned())
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

        let seen = tokio::time::timeout(std::time::Duration::from_secs(5), seen).await;
        assert_eq!(seen.expect("no heartbeat signal").unwrap(), (id, Status::Up, 1));
    }

    async fn log_010_the_heartbeat_signal_carries_the_message_that_was_stored(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        // Nothing listens on port 1, so the attempt fails with a message.
        let id = add(&pool, device, tcp(1), 0).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let bus = Bus::new(16);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        let signalled = loop {
            if let Signal::Heartbeat { beat, .. } = receiver.try_recv().expect("a heartbeat signal") {
                break beat.message;
            }
        };
        let stored: Option<String> =
            sqlx::query_scalar("SELECT message FROM checks.heartbeats WHERE check_id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(stored.is_some(), "the failed attempt has a message");
        assert_eq!(signalled, stored);
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
        let up = event("info", "docker-host is Up");
        assert_eq!(events_of(&pool, device).await, std::slice::from_ref(&up));

        sqlx::query(
            "UPDATE checks.checks SET config = jsonb_set(config, '{port}', to_jsonb($2)) \
             WHERE id = $1",
        )
        .bind(id)
        .bind(i32::from(closed_port()))
        .execute(&pool)
        .await
        .unwrap();
        let down_at = now + TimeDelta::seconds(20);
        scheduler.tick(down_at, &mut rng()).await;
        scheduler.finish_running(down_at).await;

        assert_eq!(device_statuses(&drain(&mut receiver)), [Status::Down]);
        assert_eq!(events_of(&pool, device).await, [up, event("error", "docker-host is Down")]);
    }

    async fn log_004_a_status_event_that_cannot_be_written_is_retried_before_its_signal(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 0).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);
        fail_event_writes(&pool).await;

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;
        assert_eq!(device_statuses(&drain(&mut receiver)).len(), 0);
        assert_eq!(events_of(&pool, device).await.len(), 0);
        allow_event_writes(&pool).await;
        scheduler.tick(now + TimeDelta::seconds(4), &mut rng()).await;
        assert_eq!(device_statuses(&drain(&mut receiver)).len(), 0, "retried before 5 s");
        scheduler.tick(now + TimeDelta::seconds(5), &mut rng()).await;

        assert_eq!(device_statuses(&drain(&mut receiver)), [Status::Up]);
        assert_eq!(events_of(&pool, device).await, [event("info", "docker-host is Up")]);
    }

    async fn log_004_a_push_between_ticks_gives_one_event_and_one_signal(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let push = Definition::Push(PushToken::from_random(&[5; 32]));
        let id = add(&pool, device, push, 1).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now + TimeDelta::hours(1)).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);
        scheduler.tick(now, &mut rng()).await;

        let pushed_at = now + TimeDelta::seconds(1);
        let beat = Beat {
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
            domain_not_after: None,
        };
        let mut conn = pool.acquire().await.unwrap();
        let written = checks::record_beat(&mut conn, id, &beat).await.unwrap();
        assert_eq!(written, checks::Recorded::Written);
        scheduler.tick(now + TimeDelta::seconds(2), &mut rng()).await;
        scheduler.tick(now + TimeDelta::seconds(3), &mut rng()).await;

        assert_eq!(device_statuses(&drain(&mut receiver)), [Status::Up]);
        assert_eq!(events_of(&pool, device).await, [event("info", "docker-host is Up")]);
    }
}

async fn heartbeats(pool: &PgPool, check: CheckId) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM checks.heartbeats WHERE check_id = $1")
        .bind(check)
        .fetch_one(pool)
        .await
        .unwrap()
}

tick_test! {
    async fn not_001_a_scheduled_down_beat_queues_one_notification_for_the_selected_channel(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(closed_port()), 0).await;
        notify_on(&pool, id).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;
        let later = now + TimeDelta::seconds(10);
        scheduler.tick(later, &mut rng()).await;
        scheduler.finish_running(later).await;

        assert_eq!(heartbeats(&pool, id).await, 2);
        assert_eq!(queued(&pool, id).await, ["down"], "one Down, not one per beat");
    }

    async fn not_001_a_down_beat_under_maintenance_queues_nothing(pool: PgPool) {
        // jellyfin's manual maintenance flag is set (Section 6.5).
        let device = on_loopback(&pool, JELLYFIN).await;
        let id = add(&pool, device, tcp(closed_port()), 0).await;
        notify_on(&pool, id).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        assert_eq!(heartbeats(&pool, id).await, 1);
        assert_eq!(queued(&pool, id).await, Vec::<String>::new());
    }

    async fn d_16_a_rolled_back_attempt_leaves_no_notification(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(1), 0).await;
        notify_on(&pool, id).await;
        let device = devices::get(&pool, device).await.unwrap();
        let result = ProbeResult::failed(&Failure::NoPush);
        let attempt = Attempt {
            result: &result,
            in_maintenance: false,
            at: database_now(&pool).await,
            domain: None,
            device: &device,
        };

        let mut tx = pool.begin().await.unwrap();
        let beat = checks::record_attempt(&mut tx, id, &attempt).await.unwrap();
        assert_eq!(beat.map(|beat| beat.status), Some(Status::Down));
        let inside: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications.outbox")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        tx.rollback().await.unwrap();

        assert_eq!(inside, 1, "queued in the beat's transaction");
        assert_eq!(heartbeats(&pool, id).await, 0);
        assert_eq!(queued(&pool, id).await, Vec::<String>::new());
    }
}
