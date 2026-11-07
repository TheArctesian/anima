//! `Query.events`: newest first, cursor pages, filters (LOG-004, API-006).

use anima_hub::{
    events::{self, NewEvent, Severity, Source},
    ids::DeviceId,
};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::pages::{EVENTS, HUB_EVENTS, instant, walk};
use crate::hub::{Hub, set_operator, start};

pub const ROUTER: &str = "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d";
const SWITCH: &str = "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8e";
/// The event `login` records at `start()`.
const SIGNED_IN: &str = "Operator operator signed in.";

fn hours_ago(hours: i64) -> DateTime<Utc> {
    start() - TimeDelta::hours(hours)
}

/// Records one event and returns its id as text.
async fn record(
    pool: &PgPool,
    hub: &Hub,
    at: DateTime<Utc>,
    (severity, source, device, message): (Severity, Source, Option<&str>, &str),
) -> String {
    let mut conn = pool.acquire().await.unwrap();
    let device = device.map(|id| (id.parse::<DeviceId>().unwrap(), format!("device-{id}")));
    let event = NewEvent {
        severity,
        source,
        device,
        message: message.to_owned(),
    };
    events::record(&mut conn, &hub.bus, at, event)
        .await
        .unwrap()
        .to_string()
}

/// A signed-in hub with four events recorded after the sign-in event, oldest first.
async fn filter_fixtures(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    for (hours, event) in [
        (3, (Severity::Info, Source::Check, Some(ROUTER), "probe ok")),
        (
            2,
            (Severity::Warning, Source::Access, None, "address refused"),
        ),
        (
            1,
            (Severity::Error, Source::Check, Some(SWITCH), "switch down"),
        ),
        (
            0,
            (Severity::Error, Source::Hub, Some(ROUTER), "router lost"),
        ),
    ] {
        record(pool, &hub, hours_ago(hours), event).await;
    }
    (hub, token)
}

async fn messages(hub: &Hub, token: &str, filter: Value) -> Vec<String> {
    let response = hub
        .post(Some(token), EVENTS, json!({ "filter": filter }))
        .await;
    let edges = response["data"]["events"]["edges"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"));
    edges
        .iter()
        .map(|edge| edge["node"]["message"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_004_events_filter_by_severity_source_device_and_since(pool: PgPool) {
    let (hub, token) = filter_fixtures(&pool).await;
    let since = hours_ago(2).to_rfc3339();
    // "router lost" and the sign-in share an instant; the later id comes first.
    let all = [
        "router lost",
        SIGNED_IN,
        "switch down",
        "address refused",
        "probe ok",
    ];
    for (filter, expected) in [
        (json!(null), all.to_vec()),
        (json!({}), all.to_vec()),
        (
            json!({ "severity": ["ERROR"] }),
            vec!["router lost", "switch down"],
        ),
        (
            json!({ "severity": ["WARNING", "ERROR"] }),
            vec!["router lost", "switch down", "address refused"],
        ),
        (
            json!({ "source": ["CHECK"] }),
            vec!["switch down", "probe ok"],
        ),
        (
            json!({ "source": ["ACCESS", "HUB"] }),
            vec!["router lost", SIGNED_IN, "address refused"],
        ),
        (
            json!({ "deviceId": ROUTER }),
            vec!["router lost", "probe ok"],
        ),
        (json!({ "since": since }), all[..4].to_vec()),
        (
            json!({ "severity": ["ERROR"], "deviceId": ROUTER }),
            vec!["router lost"],
        ),
        (json!({ "severity": [] }), vec![]),
        (json!({ "source": [] }), vec![]),
    ] {
        let listed = messages(&hub, &token, filter.clone()).await;
        assert_eq!(listed, expected, "{filter}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_004_an_event_shows_every_k1_field(pool: PgPool) {
    let (hub, token) = filter_fixtures(&pool).await;
    let response = hub.post(Some(&token), EVENTS, json!({ "first": 1 })).await;
    let node = &response["data"]["events"]["edges"][0]["node"];
    assert_eq!(instant(&node["at"]), start());
    assert_eq!(node["severity"], "ERROR");
    assert_eq!(node["source"], "HUB");
    assert_eq!(node["deviceId"], ROUTER);
    assert_eq!(node["deviceName"], format!("device-{ROUTER}"));
    assert_eq!(node["message"], "router lost");
    assert!(node["id"].as_str().unwrap().parse::<uuid::Uuid>().is_ok());
    let warning = hub
        .post(
            Some(&token),
            EVENTS,
            json!({ "filter": { "severity": ["WARNING"] } }),
        )
        .await;
    let node = &warning["data"]["events"]["edges"][0]["node"];
    assert_eq!(
        (&node["deviceId"], &node["deviceName"]),
        (&Value::Null, &Value::Null)
    );
    assert_eq!(node["source"], "ACCESS");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_event_pages_are_newest_first_without_gaps_or_repeats(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let event = (Severity::Info, Source::Hub, None, "tick");
    let mut stored = Vec::new();
    // Two events share an instant: the id decides their order.
    for hours in [4, 2, 2, 1, 0] {
        let at = hours_ago(hours);
        let id = record(&pool, &hub, at, event).await;
        stored.push((at, id.parse::<uuid::Uuid>().unwrap()));
    }
    stored.sort_by(|a, b| b.cmp(a));
    let newest_first: Vec<String> = stored.iter().map(|(_, id)| id.to_string()).collect();

    for (first, more) in [
        (1, vec![true, true, true, true, false]),
        (2, vec![true, true, false]),
        (4, vec![true, false]),
        (5, vec![false]),
        (200, vec![false]),
    ] {
        let walked = walk(&hub, &token, (HUB_EVENTS, "events"), first).await;
        assert_eq!(walked, (newest_first.clone(), more), "first: {first}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_a_page_after_the_last_entry_is_empty(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let empty = json!({ "edges": [], "pageInfo": { "hasNextPage": false, "endCursor": null } });
    let none = hub.post(Some(&token), HUB_EVENTS, json!({})).await;
    assert_eq!(none["data"]["events"], empty);
    let only = (Severity::Info, Source::Hub, None, "only");
    record(&pool, &hub, start(), only).await;
    let page = hub.post(Some(&token), HUB_EVENTS, json!({})).await;
    let cursor = page["data"]["events"]["pageInfo"]["endCursor"].clone();
    assert!(cursor.is_string(), "{page}");
    let after = hub
        .post(Some(&token), HUB_EVENTS, json!({ "after": cursor }))
        .await;
    assert_eq!(after["data"]["events"], empty);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_004_an_event_from_the_metrics_source_round_trips_through_the_store_and_the_api(
    pool: PgPool,
) {
    let (hub, token) = filter_fixtures(&pool).await;
    record(
        &pool,
        &hub,
        hours_ago(0),
        (
            Severity::Warning,
            Source::Metrics,
            Some(ROUTER),
            "metrics failing",
        ),
    )
    .await;

    let response = hub
        .post(
            Some(&token),
            EVENTS,
            json!({ "filter": { "source": ["METRICS"] } }),
        )
        .await;

    let edges = response["data"]["events"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1, "{response}");
    assert_eq!(edges[0]["node"]["source"], "METRICS");
    assert_eq!(edges[0]["node"]["message"], "metrics failing");
}
