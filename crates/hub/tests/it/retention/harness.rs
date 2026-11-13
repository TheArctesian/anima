//! The retention GraphQL part with a test clock, and requests from a signed-in operator.

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Bus, ClientAddr, Operator},
        retention::graphql::{Mutation, Query},
    },
    ids::OperatorId,
};
use async_graphql::{EmptySubscription, Request, Response, Schema, Variables};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;

pub type TestSchema = Schema<Query, Mutation, EmptySubscription>;

pub const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
pub const CLIENT_ADDR: &str = "10.10.0.77";

/// The instant the default test clock reports.
pub fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

pub fn schema_for(pool: &PgPool) -> TestSchema {
    let bus = Bus::new(16);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .data(bus)
        .data(clock)
        .finish()
}
/// A request from the K6 operator at `CLIENT_ADDR`, with an identity that holds well past the
/// test clock.
pub fn signed_in(query: &str) -> Request {
    let operator = Operator {
        id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
        actor: anima_hub::fleet::events::Actor::for_tests(),
    };
    Identity::for_tests(operator, now() + TimeDelta::days(3650))
        .attach(Request::new(query))
        .data(ClientAddr(CLIENT_ADDR.parse().unwrap()))
}

/// The data of a signed-in request that must have no errors.
pub async fn data(schema: &TestSchema, query: &str) -> Value {
    let response = schema.execute(signed_in(query)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

/// Runs a signed-in request with variables and returns the whole response.
pub async fn with_variables(schema: &TestSchema, query: &str, variables: Value) -> Response {
    let variables = Variables::from_json(variables);
    schema.execute(signed_in(query).variables(variables)).await
}

pub fn error_json(response: &Response) -> Value {
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    serde_json::to_value(&response.errors[0]).unwrap()
}

/// Every K1 field of `RetentionPeriod`.
pub const FIELDS: &str = "subject days minDays maxDays defaultDays";

pub const SET: &str = "mutation($subject: RetentionSubject!, $days: Int!) { \
    setRetention(subject: $subject, days: $days) { subject days minDays maxDays defaultDays } }";

/// `{ retention { .. } }`'s list, which must have no errors.
pub async fn retention(schema: &TestSchema) -> Vec<Value> {
    let query = format!("{{ retention {{ {FIELDS} }} }}");
    let data = data(schema, &query).await;
    data["retention"].as_array().unwrap().clone()
}

/// The field names of a `VALIDATION` error, in order.
pub fn validation_fields(error: &Value) -> Vec<&str> {
    assert_eq!(error["extensions"]["code"], "VALIDATION", "{error}");
    error["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["field"].as_str().unwrap())
        .collect()
}
