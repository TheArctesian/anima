use std::{sync::Arc, time::Duration};

use async_graphql::{
    EmptyMutation, Object, Request, Response, Schema,
    futures_util::{Stream, StreamExt as _},
};
use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};
use uuid::Uuid;

use super::{Subscription, delivered, event_filter, next_event, paging};
use crate::{
    access::Identity,
    clock::{Clock, FixedClock},
    events::{
        Bus, EventSummary, Operator, Severity, Signal, Source,
        graphql::EventFilterInput,
        page::{Position, encode_cursor},
        retention::Log,
    },
    ids::{DeviceId, EventId, OperatorId},
    status::Status,
};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

fn event(severity: Severity, message: &str) -> EventSummary {
    EventSummary {
        id: EventId::from(Uuid::now_v7()),
        at: now(),
        severity,
        source: Source::Check,
        device: None,
        message: message.to_owned(),
    }
}

#[test]
fn log_010_a_subscriber_receives_only_events_at_or_above_its_severity() {
    let warning = event(Severity::Warning, "disk 91%");
    let signal = || Signal::Event(warning.clone());
    assert_eq!(delivered(signal(), Severity::Info), Some(warning.clone()));
    assert_eq!(
        delivered(signal(), Severity::Warning),
        Some(warning.clone())
    );
    assert_eq!(delivered(signal(), Severity::Error), None);
    let status = Signal::DeviceStatus {
        device: DeviceId::from(Uuid::now_v7()),
        status: Status::Down,
        at: now(),
    };
    assert_eq!(delivered(status, Severity::Info), None);
}

#[test]
fn api_006_paging_takes_the_k1_default_and_refuses_bad_sizes_and_cursors() {
    assert_eq!(paging(Log::Events, None, None), Ok((100, None)));
    assert_eq!(paging(Log::Audit, Some(7), None), Ok((7, None)));
    let position = Position {
        at: now(),
        id: Uuid::now_v7(),
    };
    let cursor = encode_cursor(Log::Audit, position);
    assert_eq!(
        paging(Log::Audit, Some(1), Some(&cursor)),
        Ok((1, Some(position)))
    );
    for first in [0, 201] {
        let refused = paging(Log::Events, Some(first), None).unwrap_err();
        assert_eq!(refused.0, "first");
    }
    let refused = paging(Log::Events, Some(1), Some(&cursor)).unwrap_err();
    assert_eq!(refused.0, "after");
}

#[test]
fn log_004_the_filter_parses_the_device_id_and_keeps_the_rest() {
    let device = DeviceId::from(Uuid::now_v7());
    let filter = event_filter(EventFilterInput {
        severity: Some(vec![Severity::Error]),
        source: Some(vec![]),
        device_id: Some(device.to_string().into()),
        since: Some(now()),
    })
    .unwrap();
    assert_eq!(filter.severities, Some(vec![Severity::Error]));
    assert_eq!(filter.sources, Some(vec![]));
    assert_eq!(filter.device, Some(device));
    assert_eq!(filter.since, Some(now()));
    let bad = event_filter(EventFilterInput {
        device_id: Some("lab-router".into()),
        ..EventFilterInput::default()
    });
    assert_eq!(bad.unwrap_err().0, "filter.deviceId");
}

struct Root;

#[Object]
impl Root {
    async fn ready(&self) -> bool {
        true
    }
}

fn schema(bus: &Bus) -> Schema<Root, EmptyMutation, Subscription> {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    Schema::build(Root, EmptyMutation, Subscription)
        .data(bus.clone())
        .data(clock)
        .finish()
}

fn signed_in(request: &str) -> Request {
    let operator = Operator {
        id: OperatorId::from(Uuid::now_v7()),
        name: "operator".to_owned(),
    };
    Request::new(request).data(Identity::for_tests(operator, now() + TimeDelta::days(1)))
}

async fn next(stream: &mut (impl Stream<Item = Response> + Unpin)) -> Response {
    tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("a response in time")
        .expect("the stream is open")
}

#[tokio::test]
async fn log_010_event_added_streams_events_at_or_above_min_severity() {
    let bus = Bus::new(16);
    let schema = schema(&bus);
    let query = "subscription { eventAdded(minSeverity: WARNING) { severity source message } }";
    let mut stream = schema.execute_stream(signed_in(query)).boxed();
    // The subscription takes its receiver before the first poll returns pending.
    let first = tokio::spawn(async move {
        let response = next(&mut stream).await;
        (response, stream)
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    for (severity, message) in [
        (Severity::Info, "probe ok"),
        (Severity::Warning, "disk 91%"),
        (Severity::Error, "down"),
    ] {
        bus.publish(Signal::Event(event(severity, message)));
    }
    let (response, mut stream) = first.await.unwrap();
    let data = response.data.into_json().unwrap();
    assert_eq!(
        data["eventAdded"],
        serde_json::json!({ "severity": "WARNING", "source": "CHECK", "message": "disk 91%" })
    );
    let data = next(&mut stream).await.data.into_json().unwrap();
    assert_eq!(data["eventAdded"]["message"], "down");
}

#[tokio::test]
async fn api_003_event_added_needs_a_signed_in_operator() {
    let bus = Bus::new(16);
    let query = "subscription { eventAdded { message } }";
    let mut stream = schema(&bus).execute_stream(Request::new(query)).boxed();
    let response = next(&mut stream).await;
    assert_eq!(response.errors.len(), 1, "{response:?}");
    let code = response.errors[0]
        .extensions
        .as_ref()
        .and_then(|extensions| extensions.get("code"))
        .cloned();
    assert_eq!(code, Some(async_graphql::Value::from("UNAUTHENTICATED")));
}

#[tokio::test]
async fn log_010_a_lagging_subscriber_skips_to_the_newest_events_and_ends_with_the_bus() {
    let bus = Bus::new(1);
    let receiver = bus.subscribe();
    bus.publish(Signal::Event(event(Severity::Error, "oldest")));
    bus.publish(Signal::Event(event(Severity::Error, "newest")));
    drop(bus);
    let (node, receiver) = next_event(receiver, Severity::Info).await.unwrap();
    assert_eq!(node.text(), "newest");
    assert!(next_event(receiver, Severity::Info).await.is_none());
}
