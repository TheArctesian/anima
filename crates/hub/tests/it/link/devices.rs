//! `linkDevices` and `revokeLinkDevice` through the hub (LNK-004, LOG-005).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::control::{FakeLink, KEY, api_key, code, data, hub_over, ok, one_device, written};

const DEVICES: &str =
    "{ linkDevices { key fingerprint name address path relay expiresAt streams } }";
pub const REVOKE: &str = "mutation($k: String!) { revokeLinkDevice(key: $k) }";

#[sqlx::test(migrations = "./migrations")]
async fn lnk_004_link_devices_lists_each_device_with_its_path_and_expiry(pool: PgPool) {
    let link = FakeLink::start(|_| one_device());
    let (hub, token) = hub_over(&pool, &link).await;

    let listed = data(&hub, &token, DEVICES, json!({})).await;
    assert_eq!(
        listed["linkDevices"],
        json!([{
            "key": KEY, "fingerprint": "1a2b3c4d", "name": "laptop",
            "address": "fd41:4e49:4d41::7", "path": "RELAYED", "relay": "derp-ams",
            "expiresAt": "2030-08-03T12:00:00+00:00", "streams": 2,
        }])
    );
    // A read key may list the devices, as it may read the VPN.
    let read = api_key(&hub, &token, "READ").await;
    let response = hub.post(Some(&read), DEVICES, json!({})).await;
    assert!(response["errors"].is_null(), "{response}");
}

#[sqlx::test(migrations = "./migrations")]
async fn lnk_004_revoke_link_device_closes_the_streams_and_writes_an_audit_entry(pool: PgPool) {
    let link = FakeLink::start(|request| match request["verb"].as_str() {
        Some("devices.revoke") => ok(&json!({ "streamsClosed": 3 })),
        _ => one_device(),
    });
    let (hub, token) = hub_over(&pool, &link).await;

    let revoked = data(&hub, &token, REVOKE, json!({ "k": KEY })).await;

    assert_eq!(revoked["revokeLinkDevice"], 3);
    assert_eq!(
        link.changes(),
        [json!({ "verb": "devices.revoke", "args": { "key": KEY } })]
    );
    let stored = written(&pool).await;
    assert!(stored.contains(r#""device": "laptop""#), "{stored}");
    assert!(stored.contains(r#""fingerprint": "1a2b3c4d""#), "{stored}");
    assert!(stored.contains("link delete"), "{stored}");
    assert!(stored.contains("access Revoked the device laptop (1a2b3c4d) from the link."));
    assert!(!stored.contains(KEY), "{stored}");
}

#[sqlx::test(migrations = "./migrations")]
async fn lnk_004_a_key_the_link_does_not_list_is_not_found_and_nothing_is_sent(pool: PgPool) {
    let link = FakeLink::start(|_| one_device());
    let (hub, token) = hub_over(&pool, &link).await;
    let before = written(&pool).await;

    let response = hub
        .post(Some(&token), REVOKE, json!({ "k": "nodekey:00" }))
        .await;

    assert_eq!(code(&response), "NOT_FOUND", "{response}");
    assert_eq!(link.changes(), Vec::<Value>::new());
    assert_eq!(written(&pool).await, before);
}
