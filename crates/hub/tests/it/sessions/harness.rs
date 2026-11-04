//! A schema with the sessions part only, and requests from a signed-in operator.

use std::sync::Arc;

use anima_hub::{
    access::Identity,
    clock::{Clock, FixedClock},
    events::{ClientAddr, Operator},
    ids::OperatorId,
    sessions::{
        HostKey, Live,
        graphql::{Mutation, Query},
    },
};
use async_graphql::{EmptySubscription, Request, Response, Schema};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;

use super::{OPERATOR_ID, now};

pub type TestSchema = Schema<Query, Mutation, EmptySubscription>;

/// The address every signed-in test request comes from.
pub const CLIENT_ADDR: &str = "10.10.0.77";

/// The schema with a clock an hour after the sessions start.
pub fn schema_for(pool: &PgPool) -> TestSchema {
    schema_at(pool, now() + TimeDelta::hours(1), &Live::default())
}

/// The schema with a clock fixed at `at`, sharing `live`. `HostKey` is registered because no
/// root field returns it yet (WP-10 T3 adds them).
pub fn schema_at(pool: &PgPool, at: DateTime<Utc>, live: &Live) -> TestSchema {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(at));
    Schema::build(Query, Mutation, EmptySubscription)
        .register_output_type::<HostKey>()
        .data(pool.clone())
        .data(clock)
        .data(live.clone())
        .finish()
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
    Identity::for_tests(operator(), now() + TimeDelta::hours(hours))
        .attach(Request::new(query))
        .data(ClientAddr(CLIENT_ADDR.parse().unwrap()))
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
