//! Tests of the agent ingest parts below the HTTP client: the throttle, the bearer header, the
//! refusal answers, how the routes answer when they cannot finish, the token and code rows at
//! their edges, the stale mark at its edges, and the turns that keep agents off the pool.

mod answering;
mod credential;
mod enrolment;
mod refusals;
mod renewal;
mod stale;
mod throttle;
mod turns;

use std::{sync::Arc, time::Duration};

use agent_core::wire::{AgentToken, EnrolmentCode};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt as _;

use super::{
    enrolment::{enrol, make_code, redeem},
    routes::agent_router_within,
};
use crate::{
    clock::{Clock, FixedClock},
    events::{Bus, ClientAddr},
    ids::DeviceId,
};

/// pve-01 of the devices fixture.
fn pve() -> DeviceId {
    "0190a000-0000-7000-8002-000000000001".parse().unwrap()
}

async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(TimeDelta::seconds(1)).unwrap()
}

/// Pushes `batch` with `token` at `now` from a permitted address; the status.
async fn push(pool: &PgPool, now: DateTime<Utc>, token: &AgentToken, batch: &Value) -> StatusCode {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let router = agent_router_within(pool.clone(), Bus::new(16), clock, Duration::from_secs(10));
    let mut request = Request::post("/agent/v1/push")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token.expose()))
        .body(Body::from(batch.to_string()))
        .unwrap();
    let client = ClientAddr("100.64.0.7".parse().unwrap());
    request.extensions_mut().insert(client);
    router.oneshot(request).await.unwrap().status()
}

/// pve-01 enrolled at `now` with the token spelled by `seed`; the token as an agent presents it.
async fn enrolled_pve(pool: &PgPool, now: DateTime<Utc>, seed: u8) -> AgentToken {
    enrolled(pool, pve(), now, seed).await
}

/// `device` enrolled at `now` with the token spelled by `seed`; the token as an agent presents
/// it.
async fn enrolled(pool: &PgPool, device: DeviceId, now: DateTime<Utc>, seed: u8) -> AgentToken {
    let code = EnrolmentCode::from_random([seed; 16]);
    let presented = EnrolmentCode::parse(code.expose()).unwrap();
    let mut conn = pool.acquire().await.unwrap();
    make_code(&mut conn, device, code, now).await.unwrap();
    drop(conn);
    let redeemed = redeem(pool, &presented, now).await.unwrap().unwrap();
    let token = AgentToken::from_random([seed; 32]);
    let kept = AgentToken::parse(token.expose()).unwrap();
    let enrolled = enrol(pool, &Bus::new(4), (redeemed, token), now)
        .await
        .unwrap();
    assert!(enrolled.is_some());
    kept
}

/// The stored hashes of pve-01's current and renewed tokens.
async fn token_hashes(pool: &PgPool) -> (Vec<u8>, Option<Vec<u8>>) {
    sqlx::query_as(
        "SELECT token_sha256, next_token_sha256 FROM metrics.agents WHERE device_id = $1",
    )
    .bind(pve())
    .fetch_one(pool)
    .await
    .unwrap()
}
