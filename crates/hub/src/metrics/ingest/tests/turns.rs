//! MET-017, K8 Section 6: an agent request past its turn is `UNAVAILABLE` at once, before it
//! waits for the database. One push or renewal per device at a time, so one token cannot queue
//! pushes on its device row; at most `AGENT_REQUESTS` agent requests at once, so agents never
//! hold the connections the rest of the hub needs.

use std::{sync::Arc, time::Duration};

use agent_core::wire::AgentToken;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use http_body_util::BodyExt as _;
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::task::JoinHandle;
use tower::ServiceExt as _;

use super::{database_now, enrolled, enrolled_pve, enrolment::waits_for_locks, pve};
use crate::{
    clock::{Clock, FixedClock},
    events::{Bus, ClientAddr},
    ids::DeviceId,
    metrics::ingest::{
        renewal::renew,
        routes::agent_router_within,
        turns::{AGENT_REQUESTS, Turns},
    },
};

/// docker-host of the devices fixture.
fn docker_host() -> DeviceId {
    "0190a000-0000-7000-8002-000000000002".parse().unwrap()
}

/// A batch of `device` with one CPU point at `now`.
fn batch(device: DeviceId, now: DateTime<Utc>) -> String {
    let at = now.duration_trunc(TimeDelta::seconds(10)).unwrap();
    json!({
        "device": device.to_string(),
        "series": [{ "kind": "cpu_percent" }],
        "points": [{ "at": at.format("%Y-%m-%dT%H:%M:%SZ").to_string(), "mean": [5.0] }],
    })
    .to_string()
}

/// Pushes `body` with `token` through `router`; the status and the body text.
async fn send(router: Router, token: String, body: String) -> (StatusCode, String) {
    let mut request = Request::post("/agent/v1/push")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(body))
        .unwrap();
    let client = ClientAddr("100.64.0.7".parse().unwrap());
    request.extensions_mut().insert(client);
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn spawn_push(
    router: &Router,
    token: &AgentToken,
    body: String,
) -> JoinHandle<(StatusCode, String)> {
    tokio::spawn(send(router.clone(), token.expose().to_owned(), body))
}

fn router(pool: &PgPool, now: DateTime<Utc>) -> Router {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    agent_router_within(pool.clone(), Bus::new(16), clock, Duration::from_secs(10))
}

/// Holds the row `sql` selects for `id` in a transaction of its own until it is dropped.
async fn hold_row(
    pool: &PgPool,
    sql: &str,
    id: DeviceId,
) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut holder = pool.begin().await.unwrap();
    sqlx::query(sql)
        .bind(id)
        .execute(&mut *holder)
        .await
        .unwrap();
    holder
}

fn unavailable() -> (StatusCode, String) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        r#"{"error":"UNAVAILABLE"}"#.to_owned(),
    )
}

/// What the request answered within two seconds; a request that would wait for the held row
/// fails the test here instead of queueing.
async fn at_once(answer: impl Future<Output = (StatusCode, String)>) -> (StatusCode, String) {
    tokio::time::timeout(Duration::from_secs(2), answer)
        .await
        .expect("answered at once, without waiting for the held row")
}

#[test]
fn met_017_a_device_has_one_turn_at_a_time_and_gets_it_back_when_it_ends() {
    let turns = Turns::new(AGENT_REQUESTS);
    let first = turns.device(pve()).expect("a free turn");
    assert!(turns.device(pve()).is_none(), "taken by the first push");
    let other = turns.device(docker_host());
    assert!(other.is_some(), "another device is not held up");
    drop(first);
    assert!(
        turns.device(pve()).is_some(),
        "free again once the first ended"
    );
}

#[test]
fn met_017_at_most_the_set_number_of_requests_at_once() {
    let turns = Turns::new(AGENT_REQUESTS);
    let held: Vec<_> = (0..AGENT_REQUESTS)
        .map(|_| turns.request().unwrap())
        .collect();
    assert!(turns.request().is_none());
    drop(held);
    assert!(turns.request().is_some());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_a_second_push_of_a_device_is_unavailable_at_once_and_others_are_not_held_up(
    pool: PgPool,
) {
    let now = database_now(&pool).await;
    let token = enrolled_pve(&pool, now, 1).await;
    let neighbour = enrolled(&pool, docker_host(), now, 2).await;
    let router = router(&pool, now);
    let device_row = "SELECT 1 FROM devices.devices WHERE id = $1 FOR UPDATE";
    let holder = hold_row(&pool, device_row, pve()).await;
    let first = spawn_push(&router, &token, batch(pve(), now));
    assert!(
        waits_for_locks(&pool, &first, 1).await,
        "the first push waits for pve-01's row"
    );

    let second = send(router.clone(), token.expose().to_owned(), batch(pve(), now));
    assert_eq!(at_once(second).await, unavailable());
    let other = send(
        router.clone(),
        neighbour.expose().to_owned(),
        batch(docker_host(), now),
    );
    assert_eq!(at_once(other).await.0, StatusCode::NO_CONTENT);

    holder.rollback().await.unwrap();
    assert_eq!(first.await.unwrap().0, StatusCode::NO_CONTENT);
    let next = send(router.clone(), token.expose().to_owned(), batch(pve(), now));
    assert_eq!(
        next.await.0,
        StatusCode::NO_CONTENT,
        "the turn ended with the push"
    );
}

/// The reviewer's pool probe at its worst: ten pushes with a renewed token wait to promote it
/// while a write holds the agent row, before their device is known. Only `AGENT_REQUESTS` of
/// them may wait; the rest are `UNAVAILABLE`, and a ten-connection pool keeps room.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_agent_requests_never_hold_more_than_their_share_of_the_pool(pool: PgPool) {
    let now = database_now(&pool).await;
    let current = enrolled_pve(&pool, now, 1).await;
    let next = AgentToken::from_random([9; 32]);
    let presented = AgentToken::parse(next.expose()).unwrap();
    renew(&pool, &current, next, now).await.unwrap().unwrap();
    let small = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(1))
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let router = router(&small, now);
    let agent_row = "SELECT 1 FROM metrics.agents WHERE device_id = $1 FOR UPDATE";
    let holder = hold_row(&pool, agent_row, pve()).await;

    let pushes: Vec<_> = (0..10)
        .map(|_| spawn_push(&router, &presented, batch(pve(), now)))
        .collect();
    let wait = Duration::from_secs(5);
    let answered = tokio::time::timeout(wait, async {
        while pushes.iter().filter(|push| push.is_finished()).count() < 10 - AGENT_REQUESTS {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    assert!(answered.is_ok(), "more than {AGENT_REQUESTS} pushes waited");
    assert!(
        small.acquire().await.is_ok(),
        "the agents took the whole pool"
    );

    holder.rollback().await.unwrap();
    let mut statuses = Vec::new();
    for push in pushes {
        statuses.push(push.await.unwrap().0);
    }
    let refused = statuses
        .iter()
        .filter(|status| **status == StatusCode::SERVICE_UNAVAILABLE)
        .count();
    assert!(refused >= 10 - AGENT_REQUESTS, "{statuses:?}");
    assert!(statuses.contains(&StatusCode::NO_CONTENT), "{statuses:?}");
}
