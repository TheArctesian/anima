//! MET-017, K8 Section 6: an agent request that gets no turn is `UNAVAILABLE` before it waits
//! for the database. One push or renewal per device, so one token cannot queue pushes on its
//! device row; two requests per address until their credential is known, so one node's junk
//! cannot keep honest agents out; at most `AGENT_REQUESTS` using the database at once, waited
//! for in order, and never held while a body arrives.

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
use tokio::{task::JoinHandle, time::Instant};
use tower::ServiceExt as _;

use super::{database_now, enrolled, enrolled_pve, enrolment::waits_for_locks, pve};
use crate::{
    clock::{Clock, FixedClock},
    events::{Bus, ClientAddr},
    ids::DeviceId,
    metrics::ingest::{
        renewal::renew,
        routes::agent_router_within,
        turns::{ADDRESS_REQUESTS, AGENT_REQUESTS, REQUEST_WAIT, Turns},
    },
};

mod floods;

/// Device `n` of the devices fixture: 1 is pve-01, 2 docker-host.
fn device(n: u8) -> DeviceId {
    format!("0190a000-0000-7000-8002-0000000000{n:02x}")
        .parse()
        .unwrap()
}

/// The VPN address `100.64.0.<n>`.
fn address(n: u8) -> ClientAddr {
    ClientAddr(format!("100.64.0.{n}").parse().unwrap())
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

/// Posts `body` to `path` from `from`, with `bearer` when there is one; the status and body.
async fn post(
    router: Router,
    (path, from): (&str, ClientAddr),
    bearer: Option<String>,
    body: Body,
) -> (StatusCode, String) {
    let mut request = Request::post(path).header("content-type", "application/json");
    if let Some(bearer) = bearer {
        request = request.header("authorization", format!("Bearer {bearer}"));
    }
    let mut request = request.body(body).unwrap();
    request.extensions_mut().insert(from);
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

/// Pushes `body` with `token` from `from`.
async fn send(
    router: Router,
    from: ClientAddr,
    token: &AgentToken,
    body: String,
) -> (StatusCode, String) {
    let bearer = Some(token.expose().to_owned());
    post(router, ("/agent/v1/push", from), bearer, Body::from(body)).await
}

fn spawn_push(
    router: &Router,
    from: ClientAddr,
    token: &AgentToken,
    body: String,
) -> JoinHandle<(StatusCode, String)> {
    let (router, token) = (router.clone(), AgentToken::parse(token.expose()).unwrap());
    tokio::spawn(async move { send(router, from, &token, body).await })
}

fn router_within(pool: &PgPool, now: DateTime<Utc>, answer_time: Duration) -> Router {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    agent_router_within(pool.clone(), Bus::new(1024), clock, answer_time)
}

fn router(pool: &PgPool, now: DateTime<Utc>) -> Router {
    router_within(pool, now, Duration::from_secs(10))
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

/// A ten-connection pool of the test's database, as the hub's, for the routes; the test's own
/// pool keeps the rows held and watches the locks.
async fn hub_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(1))
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

const AGENT_ROW: &str = "SELECT 1 FROM metrics.agents WHERE device_id = $1 FOR UPDATE";

/// pve-01 enrolled with a renewed token not yet used; that token. Its first uses wait to promote
/// it while the agent row is held, before their device is known.
async fn renewed_pve(pool: &PgPool, now: DateTime<Utc>) -> AgentToken {
    let current = enrolled_pve(pool, now, 1).await;
    let next = AgentToken::from_random([9; 32]);
    let presented = AgentToken::parse(next.expose()).unwrap();
    renew(pool, &current, next, now).await.unwrap().unwrap();
    presented
}

fn unavailable() -> (StatusCode, String) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        r#"{"error":"UNAVAILABLE"}"#.to_owned(),
    )
}

/// What the request answered within `limit`; a request still waiting fails the test here.
async fn within(
    limit: Duration,
    answer: impl Future<Output = (StatusCode, String)>,
) -> (StatusCode, String) {
    tokio::time::timeout(limit, answer)
        .await
        .expect("answered in time")
}

/// Answered at once: well inside `REQUEST_WAIT`, so without waiting for a turn or a row.
async fn at_once(answer: impl Future<Output = (StatusCode, String)>) -> (StatusCode, String) {
    within(Duration::from_millis(1500), answer).await
}

#[test]
fn met_017_a_device_has_one_turn_and_an_address_two_until_they_end() {
    let turns = Turns::new(AGENT_REQUESTS);
    let first = turns.device(pve()).expect("a free turn");
    assert!(turns.device(pve()).is_none(), "taken by the first push");
    assert!(
        turns.device(device(2)).is_some(),
        "another device is not held up"
    );
    drop(first);
    assert!(
        turns.device(pve()).is_some(),
        "free again once the first ended"
    );

    let one = address(7).0;
    let held: Vec<_> = (0..ADDRESS_REQUESTS)
        .map(|_| turns.address(one).unwrap())
        .collect();
    assert!(turns.address(one).is_none());
    assert!(turns.address(address(8).0).is_some());
    drop(held);
    assert!(turns.address(one).is_some());
}

#[tokio::test(start_paused = true)]
async fn met_017_a_request_waits_in_order_for_a_turn_up_to_its_deadline() {
    let turns = Turns::new(AGENT_REQUESTS);
    let mut held = Vec::new();
    for _ in 0..AGENT_REQUESTS {
        held.push(turns.request().await.unwrap());
    }
    let started = Instant::now();
    assert!(turns.request().await.is_none());
    assert_eq!(started.elapsed(), REQUEST_WAIT);

    let waiting = tokio::spawn({
        let turns = turns.clone();
        async move { turns.request().await.is_some() }
    });
    tokio::time::sleep(REQUEST_WAIT / 2).await;
    held.pop();
    assert!(
        waiting.await.unwrap(),
        "the turn that ended went to the waiting request"
    );
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
    let neighbour = enrolled(&pool, device(2), now, 2).await;
    let router = router(&hub_pool(&pool).await, now);
    let device_row = "SELECT 1 FROM devices.devices WHERE id = $1 FOR UPDATE";
    let holder = hold_row(&pool, device_row, pve()).await;
    let first = spawn_push(&router, address(7), &token, batch(pve(), now));
    assert!(
        waits_for_locks(&pool, &first, 1).await,
        "the first push waits for pve-01's row"
    );

    let second = send(router.clone(), address(7), &token, batch(pve(), now));
    assert_eq!(at_once(second).await, unavailable());
    let other = send(
        router.clone(),
        address(8),
        &neighbour,
        batch(device(2), now),
    );
    assert_eq!(at_once(other).await.0, StatusCode::NO_CONTENT);

    holder.rollback().await.unwrap();
    assert_eq!(first.await.unwrap().0, StatusCode::NO_CONTENT);
    let next = send(router.clone(), address(7), &token, batch(pve(), now));
    assert_eq!(
        next.await.0,
        StatusCode::NO_CONTENT,
        "the turn ended with the push"
    );
}

/// The reviewer's pool probe at its worst: ten pushes from ten nodes with a renewed token wait
/// to promote it while a write holds the agent row. Only `AGENT_REQUESTS` take a connection;
/// the rest wait for a turn without one, then are `UNAVAILABLE`, and a ten-connection pool
/// keeps room throughout.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_agent_requests_never_hold_more_than_their_share_of_the_pool(pool: PgPool) {
    let now = database_now(&pool).await;
    let presented = renewed_pve(&pool, now).await;
    let small = hub_pool(&pool).await;
    let router = router(&small, now);
    let holder = hold_row(&pool, AGENT_ROW, pve()).await;

    let pushes: Vec<_> = (0..10)
        .map(|n| spawn_push(&router, address(10 + n), &presented, batch(pve(), now)))
        .collect();
    tokio::time::sleep(REQUEST_WAIT / 2).await;
    assert!(
        small.acquire().await.is_ok(),
        "the agents took the whole pool"
    );
    let answered = tokio::time::timeout(REQUEST_WAIT * 2, async {
        while pushes.iter().filter(|push| push.is_finished()).count() < 10 - AGENT_REQUESTS {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    assert!(
        answered.is_ok(),
        "more than {AGENT_REQUESTS} pushes waited for the row"
    );
    assert!(
        small.acquire().await.is_ok(),
        "the agents took the whole pool"
    );

    holder.rollback().await.unwrap();
    let mut answers = Vec::new();
    for push in pushes {
        answers.push(push.await.unwrap());
    }
    let refused = answers.iter().filter(|answer| **answer == unavailable());
    assert!(refused.count() >= 10 - AGENT_REQUESTS, "{answers:?}");
    let stored = answers
        .iter()
        .filter(|(status, _)| *status == StatusCode::NO_CONTENT);
    assert!(stored.count() >= 1, "{answers:?}");
}
