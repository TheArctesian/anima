//! What `startSession` and `endSession` tests share: requests, ready devices, and the rows and
//! audit entries a session leaves behind.

use anima_hub::sessions::Presented;
use std::net::IpAddr;

use anima_hub::{
    events::ClientAddr,
    sessions::{Live, Redeemed},
};
use async_graphql::{Request, Variables};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    ED25519_KEY, PVE_01, add_operator,
    harness::{CLIENT_ADDR, TestSchema, signed_in_until},
    put_host_key,
};

const START: &str = "mutation($input: StartSessionInput!) { startSession(input: $input) { \
                     sessionId token tunnelPath expiresAt wakeSent } }";

const END: &str = "mutation($id: ID!) { endSession(id: $id) { id deviceName operatorName \
                   startedAt endedAt endReason } }";

/// A 1280×800 view with K1's default DPI.
pub fn display() -> Value {
    json!({ "widthPx": 1280, "heightPx": 800, "timezone": "Europe/Berlin" })
}

/// `startSession` for `device` with `display`, from the K6 operator.
pub fn start_request(device: &str, display: &Value) -> Request {
    protocol_start_request(device, "SSH", display)
}

/// `startSession` for `device` over the K1 `protocol` with `display`, from the K6 operator.
pub fn protocol_start_request(device: &str, protocol: &str, display: &Value) -> Request {
    let input = json!({ "deviceId": device, "protocol": protocol, "display": display });
    signed_in_until(START, 24).variables(Variables::from_json(json!({ "input": input })))
}

/// `endSession` for `id`, from the K6 operator.
pub fn end_request(id: &str) -> Request {
    signed_in_until(END, 24).variables(Variables::from_json(json!({ "id": id })))
}

/// The K6 operator and an accepted host key for `device`, so a session can start there.
pub async fn ready(pool: &PgPool, devices: &[&str]) {
    add_operator(pool).await;
    for device in devices {
        put_host_key(pool, device, ED25519_KEY).await.unwrap();
    }
}

/// Starts a session on `device` and returns the ticket's JSON.
pub async fn started(schema: &TestSchema, device: &str) -> Value {
    let response = schema.execute(start_request(device, &display())).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["startSession"].clone()
}

/// Starts a session on PVE-01 with its device ready.
pub async fn started_on_pve_01(pool: &PgPool, schema: &TestSchema) -> Value {
    ready(pool, &[PVE_01]).await;
    started(schema, PVE_01).await
}

pub fn client() -> ClientAddr {
    ClientAddr(CLIENT_ADDR.parse::<IpAddr>().unwrap())
}

/// Redeems the ticket's token from the operator's address at `now`.
pub fn redeem(live: &Live, ticket: &Value, now: DateTime<Utc>) -> Option<Redeemed> {
    live.redeem(
        Presented::terminal(ticket["token"].as_str().unwrap(), client()),
        now,
    )
    .ok()
}

/// One stored session row as the tests read it.
#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct StoredSession {
    pub device_name: String,
    pub operator_name: String,
    pub client: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub end_reason: Option<String>,
}

pub async fn stored(pool: &PgPool, id: &str) -> StoredSession {
    sqlx::query_as(
        "select device_name, operator_name, host(client_addr) as client, started_at, ended_at, \
                end_reason \
         from sessions.sessions where id = $1::text::uuid",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

pub async fn session_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from sessions.sessions")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// One audit entry about a session.
#[derive(Debug, PartialEq, sqlx::FromRow)]
pub struct SessionAudit {
    pub action: String,
    pub target_id: String,
    pub operator_name: String,
    pub client: String,
    pub change: Value,
}

/// The audit entries on sessions, oldest first.
pub async fn session_audits(pool: &PgPool) -> Vec<SessionAudit> {
    sqlx::query_as(
        "select action, target_id, operator_name, host(client_addr) as client, change \
         from events.audit_log where target_type = 'session' order by at, action desc",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// The `session.end` entries for one session.
pub async fn end_audits(pool: &PgPool, id: &str) -> Vec<SessionAudit> {
    session_audits(pool)
        .await
        .into_iter()
        .filter(|entry| entry.action == "end" && entry.target_id == id)
        .collect()
}

pub async fn event_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from events.event_log")
        .fetch_one(pool)
        .await
        .unwrap()
}
