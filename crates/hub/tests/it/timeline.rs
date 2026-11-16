//! `Query.timeline`: the event log as Evidence, read through the hub API (K-E3; TML-001,
//! TML-002, TML-004, TML-006, TML-007, API-006).

mod event_log;
mod refusals;

use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use anima_hub::fleet::events::{self, ActorKind, NewEvent, Severity, Source};

use crate::hub::{Hub, set_operator, start};

const TIMELINE: &str = "query($from: DateTime!, $to: DateTime!, $categories: [EvidenceCategory!], \
    $first: Int, $after: String) { timeline(from: $from, to: $to, categories: $categories, \
    first: $first, after: $after) { edges { cursor node { reference source category title start \
    end basis uncertainty supersededBy } } pageInfo { hasNextPage endCursor } } }";

/// A hub at `start()` with the fixture operator and a login token.
async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

/// Records one event `minutes` before `start()` and returns its ID as text.
async fn seed(pool: &PgPool, hub: &Hub, (source, message): (Source, &str), minutes: i64) -> String {
    let event = NewEvent {
        severity: Severity::Info,
        source,
        device: None,
        message: message.to_owned(),
        actor: ActorKind::Hub,
    };
    let mut conn = pool.acquire().await.unwrap();
    let at = start() - TimeDelta::minutes(minutes);
    events::record(&mut conn, &hub.bus, at, event)
        .await
        .unwrap()
        .to_string()
}

/// The hour that ends one minute before `start()`: it holds seeded events and not the event
/// of the login.
fn hour_before() -> (DateTime<Utc>, DateTime<Utc>) {
    (
        start() - TimeDelta::minutes(61),
        start() - TimeDelta::minutes(1),
    )
}

async fn timeline(hub: &Hub, token: &str, more: Value) -> Value {
    let (from, to) = hour_before();
    let mut variables = json!({ "from": from, "to": to });
    for (name, value) in more.as_object().into_iter().flatten() {
        variables[name] = value.clone();
    }
    hub.post(Some(token), TIMELINE, variables).await
}

fn titles(response: &Value) -> Vec<&str> {
    response["data"]["timeline"]["edges"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"))
        .iter()
        .map(|edge| edge["node"]["title"].as_str().unwrap())
        .collect()
}
