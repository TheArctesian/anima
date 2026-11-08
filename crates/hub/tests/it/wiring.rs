//! The hub as `main.rs` wires it (WP-06 H-2, WP-10 handoffs 1, 2 and 4–7): the devices and
//! sessions parts of the one schema with their loaders and the shared `Live`, the checks and
//! maintenance parts with the loaders that read checks, the stale and
//! shutdown ends of sessions, the client address, a process log without the URI, and the
//! push route behind the permitted networks (WP-09 handoff 3).

mod checks_schema;
mod devices_schema;
mod edge;
mod push;
mod serving;
mod sessions_schema;
mod uri_log;

use std::sync::Arc;

use anima_hub::{
    access::Identity,
    clock::{Clock, FixedClock},
    events::{Bus, Operator},
    ids::OperatorId,
    schema::{Dependencies, HubSchema, build_schema},
    sessions::Live,
};
use async_graphql::{Request, Variables};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;

use crate::hub::dependencies;

const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const UNKNOWN_DEVICE: &str = "0190a000-0000-7000-8002-0000000000ff";
/// The address every signed-in schema request comes from.
const CLIENT_ADDR: &str = "10.10.0.77";

/// An ed25519 key and the fingerprint `ssh-keygen -l` printed for it (throwaway; the private
/// half was deleted).
const KEY: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIPTJnBUBJyxtaF5R3VPI+WZbUsZP0DeocOVAgFsBtx/m";
const FINGERPRINT: &str = "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk";

/// The instant the hub's clock reports.
fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(FixedClock(now()))
}

/// The dependencies `main.rs` builds, on `pool`, at `now()`, with `live` as the hub's one
/// `Live`.
fn wired(pool: &PgPool, live: &Live) -> Dependencies {
    Dependencies {
        live: live.clone(),
        ..dependencies(pool, clock(), Bus::new(16))
    }
}

/// The hub's schema as `build_schema` makes it.
fn hub_schema(pool: &PgPool, live: &Live) -> HubSchema {
    build_schema(wired(pool, live))
}

/// The operator a signed-in request acts as.
async fn add_operator(pool: &PgPool) {
    sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::text::uuid, 'operator', '$argon2id$x', now())",
    )
    .bind(OPERATOR_ID)
    .execute(pool)
    .await
    .unwrap();
}

/// `KEY` as the device's accepted host key, accepted by the operator named `username`.
async fn accept_key(pool: &PgPool, device: &str, username: &str) {
    sqlx::query(
        "insert into trust.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
             first_seen_at, accepted_at, accepted_by) \
         select $1::text::uuid, 'ssh-ed25519', $2, $3, $4, $4, id \
         from access.operators where username = $5",
    )
    .bind(device)
    .bind(FINGERPRINT)
    .bind(KEY)
    .bind(now())
    .bind(username)
    .execute(pool)
    .await
    .unwrap();
}

/// `query` with `variables`, from the operator signed in for a day, at `CLIENT_ADDR`.
fn signed_in(query: &str, variables: Value) -> Request {
    let operator = Operator {
        id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
    };
    Identity::for_tests(operator, now() + TimeDelta::hours(24))
        .attach(Request::new(query).variables(Variables::from_json(variables)))
        .data(anima_hub::events::ClientAddr(CLIENT_ADDR.parse().unwrap()))
}

/// The data of a signed-in request that must have no errors.
async fn data(schema: &HubSchema, query: &str, variables: Value) -> Value {
    let response = schema.execute(signed_in(query, variables)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

/// The code of the only error of a signed-in request.
async fn error_code(schema: &HubSchema, query: &str, variables: Value) -> String {
    let response = schema.execute(signed_in(query, variables)).await;
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    let error = serde_json::to_value(&response.errors[0]).unwrap();
    error["extensions"]["code"].as_str().unwrap().to_owned()
}

/// One stored session's end, as `(ended, reason)`.
async fn session_end(pool: &PgPool, id: &str) -> (bool, Option<String>) {
    sqlx::query_as(
        "select ended_at is not null, end_reason from sessions.sessions \
         where id = $1::text::uuid",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// The client address of the session's audit entries with `action`.
async fn audit_clients(pool: &PgPool, id: &str, action: &str) -> Vec<String> {
    sqlx::query_scalar(
        "select host(client_addr) from events.audit_log \
         where target_type = 'session' and target_id = $1 and action = $2",
    )
    .bind(id)
    .bind(action)
    .fetch_all(pool)
    .await
    .unwrap()
}
