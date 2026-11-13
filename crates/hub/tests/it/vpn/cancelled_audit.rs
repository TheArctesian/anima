//! A client that goes away while the broker works: the broker's change (here, a new pre-auth
//! key) still gets its hub audit entry and event, without the key (LOG-005, LOG-007, R-055).

use std::time::Duration;

use anima_hub::fleet::{events::Signal, vpn::Feed};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::{
    broker::{nodes_value, ok, read_with, start},
    graphql::hub_reading,
};

const INVITE: &str = "mutation { createVpnInvite { key expiresAt steps } }";
const KEY: &str = "REDACTED-DROPPED-KEY";
const MADE: &str = "Made a one-hour";

/// How long the hub may take to write what the broker made; only a broken hub reaches it.
const BOUND: Duration = Duration::from_secs(20);

/// Waits until the bus carries the event of the key the broker made.
async fn until_made(signals: &mut Receiver<Signal>) {
    loop {
        match signals.recv().await {
            Ok(Signal::Event(event)) if event.message.starts_with(MADE) => return,
            Ok(_) | Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => panic!("the bus closed"),
        }
    }
}

/// The audit entries of new VPN keys, once there is one; the hub commits them after the event.
async fn audited(pool: &PgPool) -> Vec<String> {
    loop {
        let audited: Vec<String> = sqlx::query_scalar(
            "select change::text from events.audit_log \
             where target_type = 'vpn' and action = 'create'",
        )
        .fetch_all(pool)
        .await
        .unwrap();
        if !audited.is_empty() {
            return audited;
        }
    }
}

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

    let mut signals = hub.bus.subscribe();

    // The browser tab closes once the broker has the request and works on it: the request
    // future is dropped.
    tokio::select! {
        biased;
        () = fake.until_received(|line| line.contains("personal-key")) => {}
        response = hub.post(Some(&token), INVITE, json!({})) => {
            panic!("the hub answered before the broker had the request: {response}");
        }
    }
    let event = tokio::time::timeout(BOUND, until_made(&mut signals)).await;
    assert!(event.is_ok(), "no event for the key the broker made");
    let audited = tokio::time::timeout(BOUND, audited(&pool)).await;
    let audited = audited.expect("no audit entry for the key the broker made");

    let made = fake
        .seen()
        .into_iter()
        .filter(|line| line.contains("personal-key"))
        .count();
    assert_eq!(made, 1, "the broker got the request and made the key");
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
