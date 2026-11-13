//! `startSession` on a Down device (K4 §1, RA-031): the resolver reads the device's status as
//! `Device.status` does and tries the wake relay first. Here the relay has no accepted host
//! key, so the wake fails: the start still succeeds with `wakeSent` false, and the wake's own
//! Error event says why. A device that is not Down is not tried.

use anima_hub::fleet::sessions::Live;
use serde_json::Value;
use sqlx::PgPool;

use crate::sessions::{
    DOCKER_HOST, PVE_01, add_operator,
    harness::schema_at,
    now,
    starting::{display, protocol_start_request},
};

/// The sentence the wake writes when its relay has no accepted host key.
const NO_RELAY_KEY: &str = "No Wake-on-LAN packet was sent: no host key is accepted for the \
                            wake relay. Test its connection and accept its key.";

/// PVE-01 with VNC on, a MAC address, one check in `status`, and DOCKER-HOST as its site's
/// wake relay.
async fn waking_pve_01(pool: &PgPool, status: &str) {
    add_operator(pool).await;
    for statement in [
        "update devices.vnc_settings set enabled = true where device_id = $1::text::uuid",
        "update devices.devices set mac_address = 'aa:bb:cc:dd:ee:ff' where id = $1::text::uuid",
    ] {
        sqlx::query(statement)
            .bind(PVE_01)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query(
        "update devices.sites set wake_relay_device_id = $2::text::uuid \
         where id = (select site_id from devices.devices where id = $1::text::uuid)",
    )
    .bind(PVE_01)
    .bind(DOCKER_HOST)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into checks.checks (id, device_id, type, name, config, status) \
         values (gen_random_uuid(), $1::text::uuid, 'icmp', 'Ping', \
                 '{\"timeoutMs\":2000,\"ipVersion\":\"V4\"}', $2)",
    )
    .bind(PVE_01)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
}

/// A VNC start on PVE-01: the ticket's `wakeSent`.
async fn wake_sent(pool: &PgPool) -> Value {
    let schema = schema_at(pool, now(), &Live::default());
    let response = schema
        .execute(protocol_start_request(PVE_01, "VNC", &display()))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["startSession"]["wakeSent"].clone()
}

async fn event_messages(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar("select message from events.event_log")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_031_a_down_device_is_tried_and_a_failed_wake_still_starts(pool: PgPool) {
    waking_pve_01(&pool, "down").await;
    assert_eq!(wake_sent(&pool).await, false);
    assert_eq!(event_messages(&pool).await, [NO_RELAY_KEY]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_031_an_up_device_is_not_tried(pool: PgPool) {
    waking_pve_01(&pool, "up").await;
    assert_eq!(wake_sent(&pool).await, false);
    assert_eq!(event_messages(&pool).await, Vec::<String>::new());
}
