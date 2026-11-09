//! Tests of the agent ingest parts below the HTTP client: the throttle, the bearer header, the
//! refusal answers, how the routes answer when they cannot finish, and the token and code rows
//! at their edges.

mod answering;
mod credential;
mod enrolment;
mod refusals;
mod renewal;
mod throttle;

use agent_core::wire::{AgentToken, EnrolmentCode};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::enrolment::{enrol, make_code, redeem};
use crate::{events::Bus, ids::DeviceId};

/// pve-01 of the devices fixture.
fn pve() -> DeviceId {
    "0190a000-0000-7000-8002-000000000001".parse().unwrap()
}

async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// pve-01 enrolled at `now` with the token spelled by `seed`; the token as an agent presents it.
async fn enrolled_pve(pool: &PgPool, now: DateTime<Utc>, seed: u8) -> AgentToken {
    let code = EnrolmentCode::from_random([seed; 16]);
    let presented = EnrolmentCode::parse(code.expose()).unwrap();
    let mut conn = pool.acquire().await.unwrap();
    make_code(&mut conn, pve(), code, now).await.unwrap();
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
