//! The notifications GraphQL part with a test clock and vault, and requests from a signed-in
//! operator. Every secret here is synthetic.

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Bus, ClientAddr, Operator},
        notifications::graphql::{Mutation, Query, last_delivery_loader},
    },
    ids::OperatorId,
};
use async_graphql::{EmptySubscription, Request, Response, Schema, Variables};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::test_vault;

pub type TestSchema = Schema<Query, Mutation, EmptySubscription>;

pub const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
pub const CLIENT_ADDR: &str = "10.10.0.77";

/// The secrets the channels below carry; no response, audit entry or event may hold one.
pub const SECRETS: [&str; 7] = [
    "s3cr3t-path",
    "s3cr3t-query",
    "header-s3cr3t",
    "topic-s3cr3t",
    "tk_s3cr3t",
    "smtp-s3cr3t",
    "apprise-s3cr3t",
];

pub const WEBHOOK_URL: &str = "https://hooks.example.test/s3cr3t-path?token=s3cr3t-query";

/// Every K1 field of `NotificationChannel`.
pub const FIELDS: &str = "id name kind onByDefault createdAt updatedAt template { title body } \
    lastDelivery { id channelId channelName kind at ok reason code } \
    settings { __typename \
      ... on WebhookSettings { origin method headerNames } \
      ... on NtfySettings { server priority tokenStored } \
      ... on EmailSettings { host port security username from to passwordStored } \
      ... on AppriseSettings { server mode tags } }";

/// The instant a `DateTime` field holds.
pub fn instant(value: &Value) -> DateTime<Utc> {
    value.as_str().unwrap().parse().unwrap()
}

/// The instant the test clock reports.
pub fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

pub fn schema_for(pool: &PgPool) -> TestSchema {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .data(Bus::new(16))
        .data(clock)
        .data(test_vault())
        .data(last_delivery_loader(pool.clone()))
        .finish()
}

/// A request from the K6 operator at `CLIENT_ADDR`.
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

/// Runs a signed-in request with variables and returns the whole response.
pub async fn run(schema: &TestSchema, query: &str, variables: Value) -> Response {
    let variables = Variables::from_json(variables);
    schema.execute(signed_in(query).variables(variables)).await
}

/// The data of a signed-in request that must have no errors.
pub async fn data(schema: &TestSchema, query: &str, variables: Value) -> Value {
    let response = run(schema, query, variables).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

pub fn create_query() -> String {
    format!(
        "mutation($input: NotificationChannelInput!) {{ \
         createNotificationChannel(input: $input) {{ {FIELDS} }} }}"
    )
}

pub fn update_query() -> String {
    format!(
        "mutation($id: ID!, $input: NotificationChannelInput!) {{ \
         updateNotificationChannel(id: $id, input: $input) {{ {FIELDS} }} }}"
    )
}

pub const DELETE: &str = "mutation($id: ID!) { deleteNotificationChannel(id: $id) }";

/// Creates a channel, which must succeed, and returns it.
pub async fn create(schema: &TestSchema, input: Value) -> Value {
    let data = data(schema, &create_query(), json!({ "input": input })).await;
    data["createNotificationChannel"].clone()
}

/// Updates channel `id` and returns the whole response.
pub async fn update(schema: &TestSchema, id: &Value, input: Value) -> Response {
    run(schema, &update_query(), json!({ "id": id, "input": input })).await
}

/// `{ notificationChannels }`, which must have no errors.
pub async fn channels(schema: &TestSchema) -> Vec<Value> {
    let query = format!("{{ notificationChannels {{ {FIELDS} }} }}");
    let data = data(schema, &query, json!({})).await;
    data["notificationChannels"].as_array().unwrap().clone()
}

/// A webhook input with one header.
pub fn webhook(name: &str) -> Value {
    json!({
        "name": name,
        "settings": { "webhook": {
            "url": WEBHOOK_URL,
            "method": "PUT",
            "headers": [{ "name": "X-Api-Key", "value": "header-s3cr3t" }],
        } },
    })
}

pub fn error_json(response: &Response) -> Value {
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    serde_json::to_value(&response.errors[0]).unwrap()
}

/// The field names of the one `VALIDATION` error of `response`, in order.
pub fn validation_fields(response: &Response) -> Vec<String> {
    let error = error_json(response);
    assert_eq!(error["extensions"]["code"], "VALIDATION", "{error}");
    error["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["field"].as_str().unwrap().to_owned())
        .collect()
}

/// Asserts that `text` holds none of [`SECRETS`] (LOG-007, SEC-005).
pub fn holds_no_secret(what: &str, text: &str) {
    for secret in SECRETS {
        assert!(!text.contains(secret), "{what} holds a secret");
    }
}

/// The stored row of channel `id` as text, its sealed secrets as raw bytes.
pub async fn stored_row(pool: &PgPool, id: &Value) -> (String, Vec<u8>) {
    let id: uuid::Uuid = id.as_str().unwrap().parse().unwrap();
    sqlx::query_as(
        "SELECT (to_jsonb(c) - 'secrets')::text, coalesce(secrets, ''::bytea) \
         FROM notifications.channels c WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}
