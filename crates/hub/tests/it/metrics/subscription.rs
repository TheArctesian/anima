//! `metricsAdded` reads the feed, guarded and filtered by device (API-003, MET-005, D-9).
//! No database is involved: the subscription reads only the feed.

use std::{sync::Arc, time::Duration};

use crate::hub::dependencies;
use anima_hub::{
    access::Identity,
    clock::FixedClock,
    events::Operator,
    ids::OperatorId,
    metrics::{Feed, MetricKind, MetricSample, MetricsAdded, Readings, graphql::Subscription},
};
use async_graphql::{
    EmptyMutation, Request, Response, Schema, SimpleObject,
    futures_util::{Stream, StreamExt as _},
};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};

#[derive(Default, SimpleObject)]
struct Query {
    ready: bool,
}

type TestSchema = Schema<Query, EmptyMutation, Subscription>;

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
const FIELDS: &str = "deviceId at samples { kind label value } \
                      readings { at cpuPercent memoryPercent diskPercent vramPercent vramGpu { name } }";

fn now() -> DateTime<Utc> {
    "2026-11-06T09:00:00Z".parse().unwrap()
}

fn schema_with(feed: &Feed) -> TestSchema {
    Schema::build(Query::default(), EmptyMutation, Subscription)
        .data(feed.clone())
        .data(Arc::new(FixedClock(now())) as Arc<dyn anima_hub::clock::Clock>)
        .finish()
}

fn signed_in(query: &str) -> Request {
    let operator = Operator {
        id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
    };
    Identity::for_tests(operator, now() + TimeDelta::days(3650)).attach(Request::new(query))
}

fn collection(device: &str, cpu: f64) -> MetricsAdded {
    MetricsAdded {
        device_id: device.into(),
        at: now(),
        samples: vec![MetricSample {
            kind: MetricKind::CpuPercent,
            label: String::new(),
            value: cpu,
        }],
        readings: Readings {
            at: now(),
            cpu_percent: Some(cpu),
            memory_percent: None,
            disk_percent: None,
            vram_percent: None,
            vram_gpu: None,
        },
    }
}

/// The first response of `stream` while `collections` are published again and again (the
/// stream subscribes to the feed when it is first polled).
async fn first_response(
    stream: impl Stream<Item = Response> + Unpin + Send + 'static,
    feed: &Feed,
    collections: &[MetricsAdded],
) -> Response {
    let reader = tokio::spawn(async move {
        let mut stream = stream;
        stream.next().await
    });
    let publisher = async {
        loop {
            for added in collections {
                feed.publish(added.clone());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::select! {
        done = reader => done.unwrap().expect("the stream ended"),
        () = publisher => unreachable!(),
        () = tokio::time::sleep(Duration::from_secs(10)) => panic!("no item arrived"),
    }
}

fn data(response: &Response) -> Value {
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.clone().into_json().unwrap()
}

#[tokio::test]
async fn api_003_a_signed_out_metrics_subscription_is_refused() {
    let schema = schema_with(&Feed::new());
    let query = format!("subscription {{ metricsAdded {{ {FIELDS} }} }}");
    let mut stream = Box::pin(schema.execute_stream(Request::new(query)));
    let response = stream.next().await.expect("a refusal");
    let code = &response.errors[0].extensions.as_ref().unwrap().get("code");
    assert_eq!(
        serde_json::to_value(code).unwrap(),
        json!("UNAUTHENTICATED")
    );
}

#[tokio::test]
async fn met_005_a_signed_in_subscriber_receives_a_collection_with_the_k1_fields() {
    let feed = Feed::new();
    let schema = schema_with(&feed);
    let query = format!("subscription {{ metricsAdded {{ {FIELDS} }} }}");
    let stream = Box::pin(schema.execute_stream(signed_in(&query)));
    let response = first_response(stream, &feed, &[collection(PVE_01, 42.0)]).await;
    let added = &data(&response)["metricsAdded"];
    assert_eq!(added["deviceId"], PVE_01);
    assert_eq!(
        added["samples"],
        json!([{ "kind": "CPU_PERCENT", "label": "", "value": 42.0 }])
    );
    assert_eq!(added["readings"]["cpuPercent"], 42.0);
    assert_eq!(added["readings"]["vramGpu"], Value::Null);
    let at: DateTime<Utc> = added["at"].as_str().unwrap().parse().unwrap();
    assert_eq!(at, now());
}

#[tokio::test]
async fn met_005_the_device_id_argument_drops_other_devices() {
    let feed = Feed::new();
    let schema = schema_with(&feed);
    let query =
        format!(r#"subscription {{ metricsAdded(deviceId: "{DOCKER_HOST}") {{ deviceId }} }}"#);
    let stream = Box::pin(schema.execute_stream(signed_in(&query)));
    let published = [collection(PVE_01, 1.0), collection(DOCKER_HOST, 2.0)];
    let response = first_response(stream, &feed, &published).await;
    assert_eq!(data(&response)["metricsAdded"]["deviceId"], DOCKER_HOST);
}

#[tokio::test]
async fn d_009_a_subscriber_that_falls_behind_skips_and_keeps_going() {
    let feed = Feed::new();
    let schema = schema_with(&feed);
    let query = "subscription { metricsAdded { readings { cpuPercent } } }";
    let mut stream = Box::pin(schema.execute_stream(signed_in(query)));
    let warm = async {
        loop {
            feed.publish(collection(PVE_01, 0.0));
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::select! {
        item = stream.next() => assert!(item.is_some()),
        () = warm => unreachable!(),
    }
    // The feed holds 256 collections; this overflows it while nobody reads.
    for _ in 0..600 {
        feed.publish(collection(PVE_01, 1.0));
    }
    feed.publish(collection(PVE_01, 99.0));
    let mut seen_last = false;
    while let Ok(Some(item)) = tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
        if data(&item)["metricsAdded"]["readings"]["cpuPercent"] == 99.0 {
            seen_last = true;
            break;
        }
    }
    assert!(
        seen_last,
        "the stream ended or never recovered after lagging"
    );
}

#[tokio::test]
async fn api_003_a_bad_device_id_is_a_validation_error_naming_the_field() {
    let schema = schema_with(&Feed::new());
    let query = r#"subscription { metricsAdded(deviceId: "nope") { deviceId } }"#;
    let mut stream = Box::pin(schema.execute_stream(signed_in(query)));
    let response = stream.next().await.expect("a refusal");
    let error = serde_json::to_value(&response.errors[0]).unwrap();
    assert_eq!(error["extensions"]["code"], "VALIDATION");
    assert_eq!(error["extensions"]["fields"][0]["field"], "deviceId");
}

#[tokio::test]
async fn p2_002_the_hub_schema_serves_a_collection_published_on_its_feed() {
    // The subscription reads only the feed, so the pool never connects.
    let pool = sqlx::PgPool::connect_lazy("postgres://unused@127.0.0.1:1/none").unwrap();
    let wired = dependencies(
        &pool,
        Arc::new(FixedClock(now())),
        anima_hub::events::Bus::new(1),
    );
    let feed = wired.metrics_feed.clone();
    let schema = anima_hub::schema::build_schema(wired);
    let query = "subscription { metricsAdded { deviceId readings { cpuPercent } } }";
    let stream = Box::pin(schema.execute_stream(signed_in(query)));
    let response = first_response(stream, &feed, &[collection(PVE_01, 7.0)]).await;
    let added = &data(&response)["metricsAdded"];
    assert_eq!(added["deviceId"], PVE_01);
    assert_eq!(added["readings"]["cpuPercent"], 7.0);
}
