//! Failed attempts over several ticks: retries, missed pushes, unreadable secrets and
//! maintenance (CHK-002, CHK-006, SEC-003).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, JELLYFIN, add, add_check, beats, closed_port, database_now, due_at, new_check,
    on_loopback, rng, scheduler, stored, tcp, vault_with,
};
use crate::{
    checks::{
        self, Definition, PushToken,
        config::{CheckConfig, HttpCredentials, HttpInput, HttpMethod, StatusRange},
        scheduler::tick::Scheduler,
    },
    credentials::Secret,
    events::Bus,
    ids::CheckId,
    status::Status,
};

async fn tick_at(scheduler: &mut Scheduler, at: DateTime<Utc>) {
    scheduler.tick(at, &mut rng()).await;
    scheduler.finish_running(at).await;
}

/// An active manual window on one target.
async fn window_on(pool: &PgPool, target_type: &str, target: String) {
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
         VALUES ($1, $2, $3)",
    )
    .bind(window)
    .bind(target_type)
    .bind(target)
    .execute(pool)
    .await
    .unwrap();
}

async fn last_message(pool: &PgPool, id: CheckId) -> Option<String> {
    sqlx::query_scalar(
        "SELECT message FROM checks.heartbeats WHERE check_id = $1 ORDER BY at DESC LIMIT 1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// An HTTP check to a closed local port, its password sealed with another key.
fn http_with_password() -> Definition {
    Definition::Http(HttpInput {
        url: format!("http://127.0.0.1:{}/", closed_port()),
        method: HttpMethod::Get,
        headers: Vec::new(),
        body: None,
        max_redirects: 0,
        accepted_status: vec![StatusRange { from: 200, to: 299 }],
        keyword: None,
        json_query: None,
        json_expected: None,
        credentials: HttpCredentials::Basic {
            username: "operator".to_owned(),
            password: Secret::from("pw-SECRET-4711".to_owned()),
        },
        verify_tls: true,
        timeout_ms: 1000,
    })
}

/// A closed-port check on the device in the group, with no retries.
async fn child_of(pool: &PgPool, text: &str) -> (CheckId, CheckId) {
    let device = on_loopback(pool, text).await;
    let group = Definition::Config(CheckConfig::Group);
    let group = add_check(pool, new_check(device, "Group", group, 0)).await;
    let mut child = new_check(device, "Child", tcp(closed_port()), 0);
    child.parent = Some(group);
    (group, add_check(pool, child).await)
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
        assert_eq!(last_message(&pool, id).await.as_deref(), Some("no push received"));
    }

    async fn sec_003_secrets_that_do_not_open_give_a_failed_beat(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let new = new_check(device, "Web", http_with_password(), 0);
        let mut conn = pool.acquire().await.unwrap();
        let id = checks::create_check(&mut conn, &vault_with(8), new).await.unwrap().id;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, id).await, ["down"]);
        let message = last_message(&pool, id).await;
        assert_eq!(message.as_deref(), Some("the check secrets cannot be opened"));
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

    async fn chk_006_a_window_on_the_device_records_maintenance_not_down(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(closed_port()), 0).await;
        window_on(&pool, "device", device.to_string()).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, id).await, ["maintenance"]);
    }

    async fn chk_006_a_window_on_the_check_records_maintenance(pool: PgPool) {
        let (_, child) = child_of(&pool, DOCKER_HOST).await;
        window_on(&pool, "check", child.to_string()).await;
        let now = database_now(&pool).await;
        due_at(&pool, child, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, child).await, ["maintenance"]);
    }

    async fn chk_006_a_window_on_the_group_records_maintenance_for_its_child(pool: PgPool) {
        let (group, child) = child_of(&pool, DOCKER_HOST).await;
        window_on(&pool, "check", group.to_string()).await;
        let now = database_now(&pool).await;
        due_at(&pool, child, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, child).await, ["maintenance"]);
    }
}
