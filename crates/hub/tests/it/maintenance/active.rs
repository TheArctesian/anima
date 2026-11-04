//! `active_for` and `active_targets`: which devices and checks the active windows cover (MW-003).

use anima_hub::{
    ids::{CheckId, DeviceId},
    maintenance::{active_for, active_targets},
};
use serde_json::json;
use sqlx::PgPool;

use super::harness::{DELETE, create, data_with, instant, manual_input, schema_for, switch_query};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("maintenance"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";
const PING: &str = "0190a000-0000-7000-8003-000000000001";
const OTHER_CHECK: &str = "0190a000-0000-7000-8003-000000000002";

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

fn check(text: &str) -> CheckId {
    text.parse().unwrap()
}

fn tags(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// A manual window applying to one target.
async fn manual_on(pool: &PgPool, name: &str, target_type: &str, target: &str) -> String {
    let schema = schema_for(pool);
    let mut input = manual_input(name);
    input["targets"] = json!([{ "type": target_type, "target": target }]);
    create(&schema, input).await
}

async fn switch(pool: &PgPool, id: &str, active: bool) {
    data_with(
        &schema_for(pool),
        &switch_query(),
        json!({ "id": id, "active": active }),
    )
    .await;
}

fixture_test! {
    async fn mw_003_a_tag_window_covers_devices_with_the_tag_while_it_runs(pool: PgPool) {
        let backup = tags(&["storage", "nas"]);
        let other = tags(&["infra"]);
        // 23:45 NZDT on 4 November: the overnight backup is running.
        let during = instant("2026-11-04T10:45:00Z");
        assert!(active_for(&pool, device(PVE_01), None, &backup, during).await.unwrap());
        assert!(!active_for(&pool, device(PVE_01), None, &other, during).await.unwrap());
        assert!(!active_for(&pool, device(PVE_01), None, &[], during).await.unwrap());
        // 22:00 NZDT: not yet.
        let before = instant("2026-11-04T09:00:00Z");
        assert!(!active_for(&pool, device(PVE_01), None, &backup, before).await.unwrap());
    }

    async fn mw_002_a_window_that_began_the_day_before_still_covers_after_midnight(pool: PgPool) {
        // 00:30 NZDT on 5 November.
        let after_midnight = instant("2026-11-04T11:30:00Z");
        let storage = tags(&["storage"]);
        assert!(active_for(&pool, device(PVE_01), None, &storage, after_midnight).await.unwrap());
        let ended = instant("2026-11-04T12:00:00Z");
        assert!(!active_for(&pool, device(PVE_01), None, &storage, ended).await.unwrap());
    }

    async fn mw_003_a_device_window_covers_that_device_while_it_is_switched_on(pool: PgPool) {
        let id = manual_on(&pool, "Pve freeze", "DEVICE", PVE_01).await;
        let now = instant("2026-11-04T09:00:00Z");
        assert!(!active_for(&pool, device(PVE_01), None, &[], now).await.unwrap());

        switch(&pool, &id, true).await;
        assert!(active_for(&pool, device(PVE_01), None, &[], now).await.unwrap());
        assert!(active_for(&pool, device(PVE_01), Some(check(PING)), &[], now).await.unwrap());
        assert!(!active_for(&pool, device(JELLYFIN), None, &[], now).await.unwrap());

        switch(&pool, &id, false).await;
        assert!(!active_for(&pool, device(PVE_01), None, &[], now).await.unwrap());
    }

    async fn mw_003_a_check_window_covers_that_check_and_not_the_whole_device(pool: PgPool) {
        let id = manual_on(&pool, "Ping freeze", "CHECK", PING).await;
        switch(&pool, &id, true).await;
        let now = instant("2026-11-04T09:00:00Z");
        assert!(active_for(&pool, device(PVE_01), Some(check(PING)), &[], now).await.unwrap());
        assert!(!active_for(&pool, device(PVE_01), Some(check(OTHER_CHECK)), &[], now).await.unwrap());
        assert!(!active_for(&pool, device(PVE_01), None, &[], now).await.unwrap());
    }

    async fn mw_003_a_deleted_window_covers_nothing(pool: PgPool) {
        let id = manual_on(&pool, "Pve freeze", "DEVICE", PVE_01).await;
        switch(&pool, &id, true).await;
        data_with(&schema_for(&pool), DELETE, json!({ "id": id })).await;
        let now = instant("2026-11-04T09:00:00Z");
        assert!(!active_for(&pool, device(PVE_01), None, &[], now).await.unwrap());
    }

    async fn api_005_one_active_targets_answers_for_many_devices_like_active_for(pool: PgPool) {
        let id = manual_on(&pool, "Pve freeze", "DEVICE", PVE_01).await;
        switch(&pool, &id, true).await;
        let now = instant("2026-11-04T10:45:00Z");
        let covering = active_targets(&pool, now).await.unwrap();
        let questions = [
            (PVE_01, vec![]),
            (JELLYFIN, tags(&["storage"])),
            (JELLYFIN, tags(&["infra"])),
            (JELLYFIN, vec![]),
        ];
        let mut answers = Vec::new();
        for (id, device_tags) in &questions {
            let batch = covering.covers(device(id), None, device_tags);
            let single = active_for(&pool, device(id), None, device_tags, now).await.unwrap();
            assert_eq!(batch, single, "{id} {device_tags:?}");
            answers.push(batch);
        }
        assert_eq!(answers, [true, true, false, false]);
    }
}
