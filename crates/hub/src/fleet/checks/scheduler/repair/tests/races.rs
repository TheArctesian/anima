//! A change that lands between the decision and the write wins: the locked re-check refuses
//! both writes and nothing is written (CHK-013, D-8). Each test changes one stored condition.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    super::{
        Candidate, give_up,
        rule::{Port, Protocol, is_candidate},
        switch,
    },
    support::{
        DOCKER_HOST, add_ping, audits_of, beat, context, database_now, edit, on_loopback,
        repair_events, stored, tried_at,
    },
};
use crate::{
    fleet::{
        checks::{Due, store::repairs},
        events::Bus,
    },
    ids::{CheckId, DeviceId},
    status::Status,
};

/// A Down ICMP check on docker-host that the rule chose, as it was read then.
struct Decided {
    id: CheckId,
    device: DeviceId,
    due: Due,
    seen: repairs::Seen,
    now: DateTime<Utc>,
}

async fn decide(pool: &PgPool) -> Decided {
    let device = on_loopback(pool, DOCKER_HOST, 22).await;
    let now = database_now(pool).await;
    let id = add_ping(pool, device, 0, now).await;
    beat(pool, id, Status::Down, now).await;
    let check = stored(pool, id).await;
    let due = Due {
        id,
        device,
        parent: None,
        config: Some(check.config),
        settings: check.settings,
        state: check.state,
        last_beat_at: check.last_beat_at,
    };
    let seen = repairs::facts(pool, id).await.unwrap().unwrap();
    assert!(is_candidate(&seen.facts, false));
    Decided {
        id,
        device,
        due,
        seen,
        now,
    }
}

/// Both writes refuse the decision, and neither left anything behind.
async fn assert_refused(pool: &PgPool, decided: &Decided) {
    let candidate = Candidate {
        due: &decided.due,
        seen: &decided.seen,
        device: "docker-host",
    };
    let ctx = context(pool, &Bus::new(64));
    let port = Port {
        protocol: Protocol::Ssh,
        number: 22,
    };
    assert!(!switch(&ctx, &candidate, port, decided.now).await.unwrap());
    assert!(!give_up(&ctx, &candidate, decided.now).await.unwrap());
    assert_eq!(tried_at(pool, decided.id).await, None);
    assert_eq!(repair_events(pool, decided.device).await, []);
    let audited = audits_of(pool, decided.id).await;
    assert!(audited.iter().all(|entry| entry.operator_name != "Anima"));
}

/// Runs `sql` on the decided check, as a writer other than the operator would.
async fn change(pool: &PgPool, decided: &Decided, sql: &str) {
    sqlx::query(sql)
        .bind(decided.id)
        .execute(pool)
        .await
        .unwrap();
}

repair_test! {
    async fn chk_013_an_unchanged_decision_is_written(pool: PgPool) {
        let decided = decide(&pool).await;
        let candidate = Candidate { due: &decided.due, seen: &decided.seen, device: "docker-host" };
        let ctx = context(&pool, &Bus::new(64));

        assert!(give_up(&ctx, &candidate, decided.now).await.unwrap());

        assert_eq!(tried_at(&pool, decided.id).await, Some(decided.now));
    }

    async fn chk_013_an_edit_between_the_decision_and_the_write_wins(pool: PgPool) {
        let decided = decide(&pool).await;
        edit(&pool, decided.id).await;
        assert_refused(&pool, &decided).await;
        assert_eq!(stored(&pool, decided.id).await.name, "Ping");
    }

    async fn chk_013_a_pause_between_the_decision_and_the_write_wins(pool: PgPool) {
        let decided = decide(&pool).await;
        change(&pool, &decided, "UPDATE checks.checks SET paused = true WHERE id = $1").await;
        assert_eq!(stored(&pool, decided.id).await.state.status, Status::Down);
        assert_refused(&pool, &decided).await;
    }

    async fn chk_013_a_status_reset_between_the_decision_and_the_write_wins(pool: PgPool) {
        let decided = decide(&pool).await;
        change(&pool, &decided, "UPDATE checks.checks SET status = 'unknown' WHERE id = $1").await;
        assert_refused(&pool, &decided).await;
    }

    async fn chk_013_a_new_config_between_the_decision_and_the_write_wins(pool: PgPool) {
        let decided = decide(&pool).await;
        let sql = "UPDATE checks.checks \
                   SET config = '{\"timeoutMs\": 2000, \"ipVersion\": \"V6\"}' WHERE id = $1";
        change(&pool, &decided, sql).await;
        assert_refused(&pool, &decided).await;
    }

    async fn chk_013_a_rename_between_the_decision_and_the_write_wins(pool: PgPool) {
        let decided = decide(&pool).await;
        change(&pool, &decided, "UPDATE checks.checks SET name = 'Echo' WHERE id = $1").await;
        assert_refused(&pool, &decided).await;
    }

    async fn chk_013_a_new_type_between_the_decision_and_the_write_wins(pool: PgPool) {
        let decided = decide(&pool).await;
        change(&pool, &decided, "UPDATE checks.checks SET type = 'tcp' WHERE id = $1").await;
        assert_refused(&pool, &decided).await;
    }

    async fn chk_013_a_repair_tried_between_the_decision_and_the_write_wins(pool: PgPool) {
        let decided = decide(&pool).await;
        let sql = "UPDATE checks.checks SET repair_tried_at = now() WHERE id = $1";
        change(&pool, &decided, sql).await;
        let candidate = Candidate { due: &decided.due, seen: &decided.seen, device: "docker-host" };
        let ctx = context(&pool, &Bus::new(64));
        assert!(!give_up(&ctx, &candidate, decided.now).await.unwrap());
        assert_eq!(repair_events(&pool, decided.device).await, []);
    }
}
