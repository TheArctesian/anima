//! `setSiteWakeRelay` and `Site.wakeRelay` (RA-031, D-7, LOG-005, API-005).

use std::sync::{Arc, atomic::Ordering};

use anima_hub::{
    clock::{Clock, FixedClock},
    devices::graphql::{Mutation, Query, device_loader},
};
use async_graphql::{EmptySubscription, Schema};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{HOME, MAC_OFFICE_1, OFFICE_1, PVE_01, set_relay};
use crate::devices::{
    batching::{BATCH_WINDOW, SITES, counting_pool},
    graphql_read::add_bulk,
    harness::{data, error_json, now, schema_for, validation_fields, with_variables},
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const SET: &str = "mutation($site: ID!, $device: ID) {
    setSiteWakeRelay(siteId: $site, deviceId: $device) { id name wakeRelay { id name } } }";
/// home-assistant, at home, with SSH off.
const SSH_OFF: &str = "0190a000-0000-7000-8002-000000000004";
const UNKNOWN_SITE: &str = "0190a000-0000-7000-8001-0000000000f1";
const UNKNOWN_DEVICE: &str = "0190a000-0000-7000-8002-0000000000ff";

async fn set(pool: &PgPool, site: &str, device: Option<&str>) -> async_graphql::Response {
    with_variables(
        &schema_for(pool),
        SET,
        json!({ "site": site, "device": device }),
    )
    .await
}

async fn set_ok(pool: &PgPool, site: &str, device: Option<&str>) -> Value {
    let response = set(pool, site, device).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["setSiteWakeRelay"].take()
}

/// The site's relay as stored.
async fn stored_relay(pool: &PgPool, site: &str) -> Option<String> {
    sqlx::query_scalar(
        "SELECT wake_relay_device_id::text FROM devices.sites WHERE id = $1::text::uuid",
    )
    .bind(site)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn audits(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar("SELECT to_jsonb(a) FROM events.audit_log a ORDER BY at, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn events(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar("SELECT to_jsonb(e) FROM events.event_log e ORDER BY at, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

fixture_test! {
    async fn d7_a_device_at_the_site_with_ssh_on_becomes_the_relay_once(pool: PgPool) {
        for _ in 0..2 {
            let site = set_ok(&pool, HOME, Some(PVE_01)).await;
            assert_eq!(site, json!({ "id": HOME, "name": "home",
                "wakeRelay": { "id": PVE_01, "name": "pve-01" } }));
        }
        assert_eq!(stored_relay(&pool, HOME).await.as_deref(), Some(PVE_01));
        let (audits, events) = (audits(&pool).await, events(&pool).await);
        assert_eq!((audits.len(), events.len()), (1, 1), "a repeated call writes nothing");
        assert_eq!((audits[0]["action"].clone(), audits[0]["target_type"].clone()),
            (json!("update"), json!("site")));
        assert_eq!(audits[0]["target_id"], HOME);
        assert_eq!(audits[0]["change"], json!({ "wakeRelay": { "id": PVE_01, "name": "pve-01" } }));
        assert_eq!(events[0]["message"], "Site home wake relay set to pve-01.");
        assert_eq!(events[0]["device_id"], PVE_01);
    }

    async fn d7_a_null_or_absent_device_clears_the_relay_and_audits_only_a_change(pool: PgPool) {
        set_relay(&pool, HOME, PVE_01).await;
        let site = set_ok(&pool, HOME, None).await;
        assert_eq!(site["wakeRelay"], Value::Null);
        assert_eq!(stored_relay(&pool, HOME).await, None);
        let absent = "mutation($site: ID!) { setSiteWakeRelay(siteId: $site) { wakeRelay { id } } }";
        let response = with_variables(&schema_for(&pool), absent, json!({ "site": HOME })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let (audits, events) = (audits(&pool).await, events(&pool).await);
        assert_eq!((audits.len(), events.len()), (1, 1));
        assert_eq!(audits[0]["change"], json!({ "wakeRelay": null }));
        assert_eq!(events[0]["message"], "Site home wake relay cleared.");
    }

    async fn d7_a_device_elsewhere_with_ssh_off_or_unknown_is_a_device_id_error(pool: PgPool) {
        set_relay(&pool, HOME, PVE_01).await;
        for device in [MAC_OFFICE_1, SSH_OFF, UNKNOWN_DEVICE, "pve-01"] {
            let error = error_json(&set(&pool, HOME, Some(device)).await);
            assert_eq!(validation_fields(&error), ["deviceId"], "{device}");
        }
        let both = error_json(&set(&pool, "home", Some("pve-01")).await);
        assert_eq!(validation_fields(&both), ["siteId", "deviceId"]);
        assert_eq!(stored_relay(&pool, HOME).await.as_deref(), Some(PVE_01));
        assert_eq!(audits(&pool).await.len(), 0);
    }

    async fn d7_an_unknown_site_is_not_found(pool: PgPool) {
        for device in [None, Some(PVE_01), Some(MAC_OFFICE_1)] {
            let error = error_json(&set(&pool, UNKNOWN_SITE, device).await);
            assert_eq!(error["extensions"]["code"], "NOT_FOUND", "{error}");
        }
        assert_eq!(audits(&pool).await.len(), 0);
    }

    async fn d7_another_site_keeps_its_relay(pool: PgPool) {
        set_relay(&pool, OFFICE_1, MAC_OFFICE_1).await;
        set_ok(&pool, HOME, Some(PVE_01)).await;
        set_ok(&pool, HOME, None).await;
        assert_eq!(stored_relay(&pool, OFFICE_1).await.as_deref(), Some(MAC_OFFICE_1));
    }

    async fn api_005_wake_relay_over_every_site_costs_a_constant_few_statements(pool: PgPool) {
        let (counted, statements) = counting_pool(&pool).await;
        let schema = Schema::build(Query, Mutation, EmptySubscription)
            .data(counted.clone())
            .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
            .data(device_loader(counted.clone()).delay(BATCH_WINDOW))
            .finish();
        let query = "{ sites { name wakeRelay { name } } }";
        set_relay(&pool, HOME, PVE_01).await;

        let few = data(&schema, query).await;
        let few_statements = statements.swap(0, Ordering::SeqCst);
        add_bulk(&pool, SITES, 150).await;
        // Each bulk site's relay is its first device by name.
        sqlx::query(
            "UPDATE devices.sites s SET wake_relay_device_id = (SELECT d.id FROM devices.devices d \
             WHERE d.site_id = s.id ORDER BY d.name LIMIT 1) WHERE s.name LIKE 'bulk-site-%'",
        )
        .execute(&pool)
        .await
        .unwrap();
        let many = data(&schema, query).await;
        let many_statements = statements.load(Ordering::SeqCst);

        let relays = |result: &Value| {
            result["sites"].as_array().unwrap().iter()
                .filter(|site| !site["wakeRelay"].is_null()).count()
        };
        assert_eq!((relays(&few), relays(&many)), (1, 1 + usize::try_from(SITES).unwrap()));
        // The sites, then one batch of devices for every relay. One lookup per site costs
        // at least 33 statements on this data.
        assert!(
            few_statements <= 12 && many_statements <= 12,
            "{few_statements} statements for 3 sites, {many_statements} for 33"
        );
    }
}

/// Waits until a lock request in this database is blocked.
async fn lock_waiter(pool: &PgPool) {
    for _ in 0..250 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_locks l JOIN pg_stat_activity a ON a.pid = l.pid \
             WHERE NOT l.granted AND a.datname = current_database()",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting > 0 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("the relay request never waited for a lock");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_031_a_relay_set_racing_an_uncommitted_ssh_off_sees_it_committed_and_refuses(
    pool: PgPool,
) {
    let mut change = pool.begin().await.unwrap();
    sqlx::query("UPDATE devices.devices SET updated_at = now() WHERE id = $1::text::uuid")
        .bind(PVE_01)
        .execute(&mut *change)
        .await
        .unwrap();
    sqlx::query("UPDATE devices.ssh_settings SET enabled = false WHERE device_id = $1::text::uuid")
        .bind(PVE_01)
        .execute(&mut *change)
        .await
        .unwrap();
    let request = tokio::spawn({
        let pool = pool.clone();
        async move { set(&pool, HOME, Some(PVE_01)).await }
    });
    lock_waiter(&pool).await;
    change.commit().await.unwrap();
    let response = request.await.unwrap();

    let error = error_json(&response);
    assert_eq!(validation_fields(&error), ["deviceId"]);
    assert_eq!(stored_relay(&pool, HOME).await, None);
}
