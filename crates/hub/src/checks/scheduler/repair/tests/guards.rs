//! What a repair never touches: a check that ever answered, a check the operator edited, and
//! a check that changed between the decision and the write (CHK-013, D-3, D-1, D-8).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{
    super::{Candidate, give_up, rule::Port, rule::Protocol, switch},
    support::{
        DOCKER_HOST, Listener, add_ping, audits_of, context, database_now, on_loopback,
        repair_events, scheduler, settle_briefly, silent_ping, stored, tick_at, tried_at, vault,
    },
};
use crate::{
    checks::{self, Beat, CheckChange, CheckState, Due, probe::ProbeResult, store::repairs},
    events::Bus,
    ids::CheckId,
    status::Status,
};

/// `pve-01`, `ollama` and `pihole` in the K6 fixtures: SSH on, no maintenance.
const MORE_HOSTS: [&str; 3] = [
    "0190a000-0000-7000-8002-000000000001",
    "0190a000-0000-7000-8002-000000000003",
    "0190a000-0000-7000-8002-000000000005",
];

/// Records a beat with `status` at `at` through the checks domain.
async fn beat(pool: &PgPool, id: CheckId, status: Status, at: DateTime<Utc>) {
    let state = CheckState {
        status,
        failures: u32::from(status == Status::Down),
        next_due_at: Some(at + TimeDelta::seconds(10)),
    };
    let probe = if status == Status::Down {
        ProbeResult::failed(&checks::probe::Failure::NoReply)
    } else {
        ProbeResult::up(std::time::Duration::from_millis(3))
    };
    let mut conn = pool.acquire().await.unwrap();
    let beat = Beat::of(probe, (state, status), at);
    let recorded = checks::record_beat(&mut conn, id, &beat).await.unwrap();
    assert_eq!(recorded, checks::Recorded::Written);
}

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

async fn make_due(pool: &PgPool, id: CheckId, at: DateTime<Utc>) {
    sqlx::query("UPDATE checks.checks SET next_due_at = $2 WHERE id = $1")
        .bind(id)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
}

async fn edit(pool: &PgPool, id: CheckId) {
    let change = CheckChange {
        degraded_threshold_ms: Some(2000),
        ..CheckChange::default()
    };
    let mut conn = pool.acquire().await.unwrap();
    checks::update_check(&mut conn, &vault(), id, change)
        .await
        .unwrap();
}

/// The check as the scheduler reads it when due.
async fn due_of(pool: &PgPool, id: CheckId) -> Due {
    let check = stored(pool, id).await;
    Due {
        id,
        device: check.device,
        parent: None,
        config: Some(check.config),
        settings: check.settings,
        state: check.state,
    }
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

    async fn chk_013_an_edit_between_the_decision_and_the_write_wins(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST, listener.port).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 0, now).await;
        beat(&pool, id, Status::Down, now).await;
        let due = due_of(&pool, id).await;
        let seen = repairs::facts(&pool, id).await.unwrap().unwrap();
        assert!(super::super::rule::is_candidate(&seen.facts, false));
        let candidate = Candidate { due: &due, seen: &seen, device: "docker-host" };
        let ctx = context(&pool, &Bus::new(64));
        let port = Port { protocol: Protocol::Ssh, number: listener.port };

        edit(&pool, id).await;

        assert!(!switch(&ctx, &candidate, port, now).await.unwrap());
        assert!(!give_up(&ctx, &candidate, now).await.unwrap());
        let check = stored(&pool, id).await;
        assert_eq!((check.config, check.name.as_str()), (silent_ping(), "Ping"));
        assert_eq!(tried_at(&pool, id).await, None);
        assert_eq!(repair_events(&pool, device).await, []);
        let updates = audits_of(&pool, id).await;
        assert!(updates.iter().all(|entry| entry.operator_name != "Anima"));
    }

    async fn chk_013_a_pause_between_the_decision_and_the_write_wins(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST, 22).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 0, now).await;
        beat(&pool, id, Status::Down, now).await;
        let due = due_of(&pool, id).await;
        let seen = repairs::facts(&pool, id).await.unwrap().unwrap();
        let candidate = Candidate { due: &due, seen: &seen, device: "docker-host" };
        let ctx = context(&pool, &Bus::new(64));
        let mut conn = pool.acquire().await.unwrap();
        checks::set_paused(&mut conn, id, true).await.unwrap();
        checks::set_paused(&mut conn, id, false).await.unwrap();

        assert!(!give_up(&ctx, &candidate, now).await.unwrap());

        assert_eq!(tried_at(&pool, id).await, None);
        assert_eq!(repair_events(&pool, device).await, []);
    }
}
