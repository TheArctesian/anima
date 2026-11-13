//! What a repair never touches, driven by real ticks: a check that ever answered, that the
//! operator edited, that is paused, or that is not ICMP (CHK-013, D-1, D-3).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, Listener, add_check, add_ping, beat, beats, closed_port, database_now, edit,
    make_due, on_loopback, repair_events, scheduler, settle_briefly, silent_ping, stored, tick_at,
    tried_at,
};
use crate::{
    fleet::{
        checks::{self, config::CheckConfig},
        events::Bus,
    },
    ids::CheckId,
    status::Status,
};

/// `pve-01`, `ollama` and `pihole` in the K6 fixtures: SSH on, no maintenance.
const MORE_HOSTS: [&str; 3] = [
    "0190a000-0000-7000-8002-000000000001",
    "0190a000-0000-7000-8002-000000000003",
    "0190a000-0000-7000-8002-000000000005",
];

/// One uptime row in `table` with 1 in `column`.
async fn bucket(pool: &PgPool, id: CheckId, (table, column): (&str, &str), at: DateTime<Utc>) {
    let sql = format!("INSERT INTO checks.{table} (check_id, bucket, {column}) VALUES ($1, $2, 1)");
    sqlx::query(&sql)
        .bind(id)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
}

repair_test! {
    async fn chk_013_a_check_that_ever_answered_is_never_repaired(pool: PgPool) {
        let listener = Listener::start().await;
        let now = database_now(&pool).await;
        let earlier = now - TimeDelta::minutes(2);
        let mut ids = Vec::new();
        for host in [DOCKER_HOST].into_iter().chain(MORE_HOSTS) {
            let device = on_loopback(&pool, host, listener.port).await;
            ids.push(add_ping(&pool, device, 0, now).await);
        }
        // An Up heartbeat whose buckets are gone, then one bucket of each table alone.
        beat(&pool, ids[0], Status::Up, earlier).await;
        for table in ["uptime_minute", "uptime_hour", "uptime_day"] {
            let sql = format!("DELETE FROM checks.{table} WHERE check_id = $1");
            sqlx::query(&sql).bind(ids[0]).execute(&pool).await.unwrap();
        }
        bucket(&pool, ids[1], ("uptime_minute", "up_count"), earlier).await;
        bucket(&pool, ids[2], ("uptime_hour", "degraded_count"), earlier).await;
        bucket(&pool, ids[3], ("uptime_day", "up_count"), earlier).await;
        for id in &ids {
            make_due(&pool, *id, now).await;
        }
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;
        settle_briefly().await;

        for id in ids {
            assert_eq!(stored(&pool, id).await.state.status, Status::Down);
            assert_eq!(stored(&pool, id).await.config, silent_ping());
            assert_eq!(tried_at(&pool, id).await, None);
        }
        assert_eq!(listener.accepted(), 0);
    }

    async fn chk_013_a_check_the_operator_edited_is_never_repaired(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST, listener.port).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 0, now).await;
        edit(&pool, id).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;
        settle_briefly().await;

        assert_eq!(stored(&pool, id).await.state.status, Status::Down);
        assert_eq!(stored(&pool, id).await.config, silent_ping());
        assert_eq!((listener.accepted(), tried_at(&pool, id).await), (0, None));
        assert_eq!(repair_events(&pool, device).await, []);
    }

    async fn chk_013_a_paused_check_is_never_repaired(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST, listener.port).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 0, now).await;
        let mut conn = pool.acquire().await.unwrap();
        checks::set_paused(&mut conn, id, true).await.unwrap();
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;
        tick_at(&mut scheduler, now + TimeDelta::seconds(10)).await;
        settle_briefly().await;

        assert_eq!(beats(&pool, id).await, Vec::<String>::new());
        assert_eq!(stored(&pool, id).await.config, silent_ping());
        assert_eq!((listener.accepted(), tried_at(&pool, id).await), (0, None));
        assert_eq!(repair_events(&pool, device).await, []);
    }

    async fn chk_013_a_check_that_is_not_icmp_is_never_repaired(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST, listener.port).await;
        let now = database_now(&pool).await;
        let closed = CheckConfig::Tcp { port: closed_port(), timeout_ms: 1000 };
        let id = add_check(&pool, device, ("Service port", closed.clone(), 0), now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;
        settle_briefly().await;

        assert_eq!(beats(&pool, id).await, ["down"]);
        let check = stored(&pool, id).await;
        assert_eq!((check.config, check.name.as_str()), (closed, "Service port"));
        assert_eq!((listener.accepted(), tried_at(&pool, id).await), (0, None));
        assert_eq!(repair_events(&pool, device).await, []);
    }
}
