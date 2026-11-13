//! The maintenance schema with a test clock, and requests from a signed-in operator.

use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Bus, ClientAddr, Operator},
        maintenance::graphql::{Mutation, Query},
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

pub fn instant(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

/// The instant the default test clock reports: 22:00 on Wednesday 4 November in Auckland.
pub fn now() -> DateTime<Utc> {
    instant("2026-11-04T09:00:00Z")
}

/// The schema with a clock fixed at `now`; the bus is returned so a test can subscribe.
pub fn schema_at(pool: &PgPool, now: DateTime<Utc>) -> (TestSchema, Bus) {
    schema_with_clock(pool, Arc::new(FixedClock(now)))
}

/// A clock that reports one second later on every call, so a mutation that reads it twice
/// stamps two different times.
pub struct TickingClock(AtomicI64);

impl TickingClock {
    pub fn new() -> Self {
        Self(AtomicI64::new(0))
    }
}

impl Clock for TickingClock {
    fn now(&self) -> DateTime<Utc> {
        now() + TimeDelta::seconds(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

pub fn schema_with_clock(pool: &PgPool, clock: Arc<dyn Clock>) -> (TestSchema, Bus) {
    let bus = Bus::new(16);
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .data(bus.clone())
        .data(clock)
        .finish();
    (schema, bus)
}

pub fn schema_for(pool: &PgPool) -> TestSchema {
    schema_at(pool, now()).0
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

/// The data of a signed-in request with variables that must have no errors.
pub async fn data_with(schema: &TestSchema, query: &str, variables: Value) -> Value {
    let response = with_variables(schema, query, variables).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

pub fn error_json(response: &Response) -> Value {
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    serde_json::to_value(&response.errors[0]).unwrap()
}

/// Every K1 field of `MaintenanceWindow`.
pub const FIELDS: &str = "id name kind timezone activeManual startsAt endsAt firstStartAt \
    everyDays weekdays daysOfMonth startTime durationMinutes targets { type target } activeNow";

pub fn create_query() -> String {
    format!(
        "mutation($input: MaintenanceWindowInput!) {{ createMaintenanceWindow(input: $input) {{ {FIELDS} }} }}"
    )
}

pub fn update_query() -> String {
    format!(
        "mutation($id: ID!, $input: MaintenanceWindowInput!) {{ updateMaintenanceWindow(id: $id, input: $input) {{ {FIELDS} }} }}"
    )
}

pub fn switch_query() -> String {
    format!(
        "mutation($id: ID!, $active: Boolean!) {{ setManualWindowActive(id: $id, active: $active) {{ {FIELDS} }} }}"
    )
}

pub const DELETE: &str = "mutation($id: ID!) { deleteMaintenanceWindow(id: $id) }";

/// A valid `MANUAL` window input applying to the tag `infra`.
pub fn manual_input(name: &str) -> Value {
    serde_json::json!({
        "name": name, "kind": "MANUAL", "timezone": "UTC",
        "targets": [{ "type": "TAG", "target": "infra" }],
    })
}

/// A valid `RECURRING_WEEKDAY` input: Sundays 03:00 for an hour in Auckland, applying to `infra`.
pub fn weekday_input(name: &str) -> Value {
    serde_json::json!({
        "name": name, "kind": "RECURRING_WEEKDAY", "timezone": "Pacific/Auckland",
        "weekdays": ["SUN"], "startTime": "03:00", "durationMinutes": 60,
        "targets": [{ "type": "TAG", "target": "infra" }],
    })
}

/// Creates a window with the schema and returns its `id`.
pub async fn create(schema: &TestSchema, input: Value) -> String {
    let created = data_with(
        schema,
        &create_query(),
        serde_json::json!({ "input": input }),
    )
    .await;
    created["createMaintenanceWindow"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
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
