//! `due_checks`: what the scheduler probes next (CHK-001, CHK-005).

use anima_hub::{
    checks::{self, Definition, config::CheckConfig},
    ids::CheckId,
};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{PVE_01, create, icmp, new_check};

macro_rules! devices_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

async fn due_at(pool: &PgPool, id: CheckId, at: DateTime<Utc>) {
    sqlx::query("UPDATE checks.checks SET next_due_at = $2 WHERE id = $1")
        .bind(id)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
}

async fn due_ids(pool: &PgPool, limit: u32) -> Vec<CheckId> {
    checks::due_checks(pool, now(), limit)
        .await
        .unwrap()
        .into_iter()
        .map(|due| due.id)
        .collect()
}

devices_test! {
    async fn chk_001_due_checks_are_the_unpaused_probed_checks_whose_time_has_come(pool: PgPool) {
        let fresh = create(&pool, new_check(PVE_01, "Fresh", icmp())).await;
        let late = create(&pool, new_check(PVE_01, "Late", icmp())).await;
        let exact = create(&pool, new_check(PVE_01, "Exact", icmp())).await;
        let later = create(&pool, new_check(PVE_01, "Later", icmp())).await;
        let paused = create(&pool, new_check(PVE_01, "Paused", icmp())).await;
        let group = create(&pool, new_check(PVE_01, "Group", Definition::Config(CheckConfig::Group))).await;
        due_at(&pool, late, now() - TimeDelta::minutes(5)).await;
        due_at(&pool, exact, now()).await;
        due_at(&pool, later, now() + TimeDelta::seconds(1)).await;
        let mut conn = pool.acquire().await.unwrap();
        checks::set_paused(&mut conn, paused, true).await.unwrap();

        assert_eq!(due_ids(&pool, 10).await, [fresh, late, exact]);
        assert_eq!(due_ids(&pool, 2).await, [fresh, late]);
        assert!(!due_ids(&pool, 10).await.contains(&group));
    }

    async fn chk_001_a_due_check_carries_its_settings_state_and_readable_config(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Ping", icmp())).await;
        let due = checks::due_checks(&pool, now(), 10).await.unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, id);
        assert_eq!(due[0].device, super::device(PVE_01));
        assert!(matches!(due[0].config, Some(CheckConfig::Icmp { .. })));

        // A configuration this version cannot read does not stop the schedule.
        sqlx::query("UPDATE checks.checks SET config = '{\"timeout_ms\": \"soon\"}' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        let due = checks::due_checks(&pool, now(), 10).await.unwrap();
        assert_eq!((due[0].id, due[0].config.clone()), (id, None));
    }
}
