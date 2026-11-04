//! Failed attempts over several ticks: retries, missed pushes and maintenance (CHK-002,
//! CHK-006).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, JELLYFIN, add, beats, closed_port, database_now, due_at, on_loopback, rng,
    scheduler, stored, tcp,
};
use crate::{
    checks::{Definition, PushToken, scheduler::tick::Scheduler},
    events::Bus,
    ids::DeviceId,
    status::Status,
};

async fn tick_at(scheduler: &mut Scheduler, at: DateTime<Utc>) {
    scheduler.tick(at, &mut rng()).await;
    scheduler.finish_running(at).await;
}

async fn manual_window_on(pool: &PgPool, device: DeviceId) {
    let window = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO maintenance.windows (id, name, kind, timezone, rule, active_manual) \
         VALUES ($1, 'Rack work', 'manual', 'UTC', '{\"kind\": \"manual\"}', true)",
    )
    .bind(window)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO maintenance.window_targets (window_id, target_type, target) \
         VALUES ($1, 'device', $2::text)",
    )
    .bind(window)
    .bind(device.to_string())
    .execute(pool)
    .await
    .unwrap();
}

tick_test! {
    async fn chk_002_a_closed_port_is_down_only_after_the_retries(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(closed_port()), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;
        let first = stored(&pool, id).await.state;
        assert_eq!((first.status, first.failures), (Status::Unknown, 1));
        assert_eq!(first.next_due_at, Some(now + TimeDelta::seconds(5)));
        tick_at(&mut scheduler, now + TimeDelta::seconds(4)).await;
        assert_eq!(beats(&pool, id).await, ["unknown"]);
        tick_at(&mut scheduler, now + TimeDelta::seconds(5)).await;
        assert_eq!(stored(&pool, id).await.state.status, Status::Unknown);
        tick_at(&mut scheduler, now + TimeDelta::seconds(10)).await;

        assert_eq!(beats(&pool, id).await, ["unknown", "unknown", "down"]);
        assert_eq!(stored(&pool, id).await.state.status, Status::Down);
    }

    async fn chk_002_a_push_check_with_no_push_is_down_after_its_retries(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, Definition::Push(PushToken::from_random(&[3; 32])), 1).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;
        assert_eq!(stored(&pool, id).await.state.status, Status::Unknown);
        tick_at(&mut scheduler, now + TimeDelta::seconds(5)).await;

        assert_eq!(beats(&pool, id).await, ["unknown", "down"]);
        let message: Option<String> = sqlx::query_scalar(
            "SELECT message FROM checks.heartbeats WHERE check_id = $1 ORDER BY at DESC LIMIT 1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(message.as_deref(), Some("no push received"));
    }

    async fn chk_006_a_device_with_the_manual_flag_records_maintenance_not_down(pool: PgPool) {
        let device = on_loopback(&pool, JELLYFIN).await;
        let id = add(&pool, device, tcp(closed_port()), 0).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, id).await, ["maintenance"]);
        assert_eq!(stored(&pool, id).await.state.status, Status::Down);
    }

    async fn chk_006_an_active_window_records_maintenance_not_down(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(closed_port()), 0).await;
        manual_window_on(&pool, device).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, id).await, ["maintenance"]);
    }
}
