//! A schema with the sessions part only, and requests from a signed-in operator.

use std::{io::Write as _, sync::Arc};

use anima_hub::{
    access::Identity,
    clock::{Clock, FixedClock},
    credentials::Vault,
    events::{ClientAddr, Operator},
    ids::OperatorId,
    sessions::{
        Live,
        graphql::{Mutation, Query, Subscription},
    },
};
use async_graphql::{Request, Response, Schema};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;

use super::{OPERATOR_ID, now};

pub type TestSchema = Schema<Query, Mutation, Subscription>;

/// The address every signed-in test request comes from.
pub const CLIENT_ADDR: &str = "10.10.0.77";

/// The schema with a clock an hour after the sessions start.
pub fn schema_for(pool: &PgPool) -> TestSchema {
    schema_at(pool, now() + TimeDelta::hours(1), &Live::default())
}

/// The schema with a clock fixed at `at`, sharing `live`.
pub fn schema_at(pool: &PgPool, at: DateTime<Utc>, live: &Live) -> TestSchema {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(at));
    Schema::build(Query, Mutation, Subscription)
        .data(pool.clone())
        .data(clock)
        .data(live.clone())
        .data(vault())
        .finish()
}

/// The schema's vault: a throwaway key written to a temporary file (SEC-004), never a real
/// key. Every call returns a vault with the same key.
pub fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// The K6 operator as the guard puts it in the context.
pub fn operator() -> Operator {
    Operator {
        id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
    }
}

/// A request from the K6 operator whose identity expires at `expires_at` hours after `now()`.
pub fn signed_in_until(query: &str, hours: i64) -> Request {
    signed_in_from(query, CLIENT_ADDR, hours)
}

/// A request from the K6 operator at `client`, signed in for `hours` after `now()`.
pub fn signed_in_from(query: &str, client: &str, hours: i64) -> Request {
    Identity::for_tests(operator(), now() + TimeDelta::hours(hours))
        .attach(Request::new(query))
        .data(ClientAddr(client.parse().unwrap()))
}

/// The data of a signed-in request that must have no errors.
pub async fn data(schema: &TestSchema, query: &str) -> Value {
    let response = schema.execute(signed_in_until(query, 24)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

/// The only error of a response, as JSON with `extensions`.
pub fn error_json(response: &Response) -> Value {
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    serde_json::to_value(&response.errors[0]).unwrap()
}

/// The only error of a signed-in request.
pub async fn only_error(schema: &TestSchema, query: &str) -> Value {
    error_json(&schema.execute(signed_in_until(query, 24)).await)
}
