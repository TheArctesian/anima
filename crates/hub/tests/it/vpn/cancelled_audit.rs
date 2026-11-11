//! A client that goes away while the broker works: the broker's change (here, a new pre-auth
//! key) still gets its hub audit entry and event, without the key (LOG-005, LOG-007, R-055).

use std::time::Duration;

use anima_hub::vpn::Feed;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    broker::{nodes_value, ok, read_with, start},
    graphql::hub_reading,
};

const INVITE: &str = "mutation { createVpnInvite { key expiresAt steps } }";
const KEY: &str = "REDACTED-DROPPED-KEY";

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_dropped_request_still_audits_the_key_the_broker_made(pool: PgPool) {
    // The broker takes 1 s, as Headscale does under load, then makes the key.
    let fake = start(Duration::from_secs(1), |line| {
        let request: Value = serde_json::from_str(line).unwrap();
        Some(if request["verb"] == "nodes" {
            ok(&nodes_value())
        } else {
            ok(&json!({ "key": KEY, "expiresAt": "2030-05-05T13:00:00.000Z", "keyId": 7 }))
        })
    });
    let feed = Feed::new();
    let (hub, token) = hub_reading(&pool, &feed, Some(fake.broker())).await;
    let reader = read_with(&pool, &feed, fake.broker()).await;
    reader.abort();

    // The browser tab closes 300 ms in: the request future is dropped.
    let gone = tokio::time::timeout(
        Duration::from_millis(300),
        hub.post(Some(&token), INVITE, json!({})),
    )
    .await;
    assert!(gone.is_err());
    tokio::time::sleep(Duration::from_secs(2)).await;

    let made = fake
        .seen()
        .into_iter()
        .filter(|line| line.contains("personal-key"))
        .count();
    assert_eq!(made, 1, "the broker got the request and made the key");
    let audited: Vec<String> = sqlx::query_scalar(
        "select change::text from events.audit_log where target_type = 'vpn' and action = 'create'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        audited.len(),
        1,
        "one audit entry for the key the broker made"
    );
    assert!(!audited[0].contains(KEY), "{audited:?}");
    let events: i64 = sqlx::query_scalar(
        "select count(*) from events.event_log where source = 'vpn' and message like 'Made a one-hour%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 1, "one event for the key the broker made");
}
