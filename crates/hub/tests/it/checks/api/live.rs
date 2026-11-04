//! The subscriptions read the bus and filter by id (LOG-010, E-4).

use std::time::Duration;

use anima_hub::{
    events::{Bus, HeartbeatSummary, Signal},
    ids::{CheckId, DeviceId},
    status::Status,
};
use async_graphql::futures_util::{Stream, StreamExt as _};
use serde_json::{Value, json};

use super::harness::{
    DOCKER_HOST, PVE_01, PVE_01_PING, TestSchema, now, schema_with_bus, signed_in,
};

const OTHER_CHECK: &str = "0190a000-0000-7000-8003-000000000003";

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

fn status(text: &str, status: Status) -> Signal {
    Signal::DeviceStatus {
        device: device(text),
        status,
        at: now(),
    }
}

fn heartbeat(check: &str, owner: &str) -> Signal {
    Signal::Heartbeat {
        check: check.parse::<CheckId>().unwrap(),
        device: device(owner),
        beat: HeartbeatSummary {
            at: now(),
            status: Status::Up,
            response_time_ms: Some(12),
        },
    }
}

/// The first item the subscription yields while `signals` are published again and again until
/// then (the stream subscribes to the bus when it is first polled).
async fn first_item(
    stream: impl Stream<Item = async_graphql::Response> + Unpin + Send + 'static,
    bus: &Bus,
    signals: &[Signal],
) -> Value {
    let reader = tokio::spawn(async move {
        let mut stream = stream;
        stream.next().await
    });
    let publisher = async {
        loop {
            for signal in signals {
                bus.publish(signal.clone());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    let response = tokio::select! {
        done = reader => done.unwrap().expect("the stream ended"),
        () = publisher => unreachable!(),
        () = tokio::time::sleep(Duration::from_secs(10)) => panic!("no item arrived"),
    };
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

fn subscribe(
    schema: &TestSchema,
    query: &str,
) -> impl Stream<Item = async_graphql::Response> + use<> {
    schema.execute_stream(signed_in(query))
}

#[sqlx::test(migrations = "./migrations")]
async fn log_010_device_status_changed_filters_by_device_and_carries_the_signals_time(
    pool: sqlx::PgPool,
) {
    let (schema, bus) = schema_with_bus(&pool);
    let query = format!(
        r#"subscription {{ deviceStatusChanged(deviceId: "{PVE_01}") {{ deviceId status at }} }}"#
    );
    let signals = [
        status(DOCKER_HOST, Status::Up),
        status(PVE_01, Status::Down),
    ];
    let item = first_item(Box::pin(subscribe(&schema, &query)), &bus, &signals).await;
    assert_eq!(
        item["deviceStatusChanged"],
        json!({
            "deviceId": PVE_01, "status": "DOWN", "at": "2026-11-04T09:00:00+00:00",
        })
    );
    let all = "subscription { deviceStatusChanged { deviceId } }";
    let item = first_item(Box::pin(subscribe(&schema, all)), &bus, &signals[..1]).await;
    assert_eq!(item["deviceStatusChanged"]["deviceId"], DOCKER_HOST);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_010_heartbeat_added_filters_by_check_and_by_device(pool: sqlx::PgPool) {
    let (schema, bus) = schema_with_bus(&pool);
    let noise = [
        heartbeat(OTHER_CHECK, PVE_01),
        heartbeat(PVE_01_PING, DOCKER_HOST),
        status(PVE_01, Status::Down),
    ];
    let wanted = heartbeat(PVE_01_PING, PVE_01);
    let signals = [noise[0].clone(), noise[1].clone(), noise[2].clone(), wanted];
    let query = format!(
        r#"subscription {{ heartbeatAdded(checkId: "{PVE_01_PING}", deviceId: "{PVE_01}") {{
        checkId deviceId heartbeat {{ status responseTimeMs message }} }} }}"#
    );
    let item = first_item(Box::pin(subscribe(&schema, &query)), &bus, &signals).await;
    assert_eq!(
        item["heartbeatAdded"],
        json!({
            "checkId": PVE_01_PING, "deviceId": PVE_01,
            "heartbeat": { "status": "UP", "responseTimeMs": 12, "message": null },
        })
    );
    let by_device =
        format!(r#"subscription {{ heartbeatAdded(deviceId: "{DOCKER_HOST}") {{ checkId }} }}"#);
    let item = first_item(Box::pin(subscribe(&schema, &by_device)), &bus, &signals).await;
    assert_eq!(item["heartbeatAdded"]["checkId"], PVE_01_PING);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_010_a_subscriber_that_falls_behind_skips_and_keeps_going(pool: sqlx::PgPool) {
    let (schema, bus) = schema_with_bus(&pool);
    let query = "subscription { deviceStatusChanged { status } }";
    let mut stream = Box::pin(subscribe(&schema, query));
    // The stream subscribes on its first poll; take one item to be sure it has.
    let warm = async {
        loop {
            bus.publish(status(PVE_01, Status::Up));
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::select! {
        item = stream.next() => assert!(item.is_some()),
        () = warm => unreachable!(),
    }
    // The bus holds 16 signals; this overflows it while nobody reads.
    for _ in 0..200 {
        bus.publish(status(PVE_01, Status::Up));
    }
    bus.publish(status(PVE_01, Status::Down));
    let mut seen_down = false;
    while let Ok(Some(item)) = tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
        assert!(item.errors.is_empty(), "{:?}", item.errors);
        if item.data.into_json().unwrap()["deviceStatusChanged"]["status"] == "DOWN" {
            seen_down = true;
            break;
        }
    }
    assert!(
        seen_down,
        "the stream ended or never recovered after lagging"
    );
}
