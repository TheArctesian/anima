//! The test schema as `schema.rs` builds it, and requests from a signed-in operator.

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    devices::graphql::{Mutation, Query, devices_by_site_loader, site_loader},
    events::{Bus, ClientAddr, Operator},
    ids::OperatorId,
};
use async_graphql::{EmptySubscription, Request, Response, Schema};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;

pub type TestSchema = Schema<Query, Mutation, EmptySubscription>;

pub const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
pub const CLIENT_ADDR: &str = "10.10.0.77";

/// The instant the test clock reports.
pub fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

/// The schema with a fresh bus; the bus is returned so a test can subscribe.
pub fn schema_with_bus(pool: &PgPool) -> (TestSchema, Bus) {
    let bus = Bus::new(16);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .data(site_loader(pool.clone()))
        .data(devices_by_site_loader(pool.clone()))
        .data(bus.clone())
        .data(clock)
        .finish();
    (schema, bus)
}

pub fn schema_for(pool: &PgPool) -> TestSchema {
    schema_with_bus(pool).0
}

/// A request from the K6 operator at `CLIENT_ADDR`.
pub fn signed_in(query: &str) -> Request {
    Request::new(query)
        .data(Operator {
            id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
            name: "operator".to_owned(),
        })
        .data(ClientAddr(CLIENT_ADDR.parse().unwrap()))
}

/// The data of a response that must have no errors.
pub async fn data(schema: &TestSchema, query: &str) -> Value {
    run(schema, signed_in(query)).await
}

/// The data of a signed-in request that must have no errors.
pub async fn run(schema: &TestSchema, request: Request) -> Value {
    let response = schema.execute(request).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

/// The only error of a signed-in request, as JSON with `extensions`.
pub async fn only_error(schema: &TestSchema, query: &str) -> Value {
    error_json(&schema.execute(signed_in(query)).await)
}

pub fn error_json(response: &Response) -> Value {
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    serde_json::to_value(&response.errors[0]).unwrap()
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

/// Runs a signed-in request with variables and returns the whole response.
pub async fn with_variables(schema: &TestSchema, query: &str, variables: Value) -> Response {
    let variables = async_graphql::Variables::from_json(variables);
    schema.execute(signed_in(query).variables(variables)).await
}
