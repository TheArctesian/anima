//! The web interfaces schema with a fixed clock, and requests from a signed-in operator.

use std::sync::Arc;

use anima_hub::{
    access::Identity,
    clock::FixedClock,
    events::{Bus, ClientAddr, Operator},
    ids::OperatorId,
    web_interfaces::graphql::Mutation,
};
use async_graphql::{EmptySubscription, Request, Response, Schema, SimpleObject, Variables};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

/// K1 has no root query for web interfaces; a schema needs one field.
#[derive(Default, SimpleObject)]
pub struct Query {
    ready: bool,
}

pub type TestSchema = Schema<Query, Mutation, EmptySubscription>;

pub const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
pub const CLIENT_ADDR: &str = "10.10.0.77";
pub const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
pub const ABSENT: &str = "0190a000-0000-7000-8002-0000000000ff";
/// An HTTP check on `pve-01`: `https://pve-01.home.lan:8006/`.
pub const HTTP_CHECK: &str = "0190a000-0000-7000-8003-00000000001a";
pub const PING_CHECK: &str = "0190a000-0000-7000-8003-000000000001";
pub const FINGERPRINT: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Every K1 field of `WebInterface`.
pub const FIELDS: &str = "id deviceId label url openIn pinnedFingerprintSha256 pinnedAt";

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}
pub(crate) use fixture_test;

pub fn now() -> DateTime<Utc> {
    "2026-11-06T09:00:00Z".parse().unwrap()
}

pub fn schema_for(pool: &PgPool) -> TestSchema {
    Schema::build(Query::default(), Mutation, EmptySubscription)
        .data(pool.clone())
        .data(Bus::new(16))
        .data(Arc::new(FixedClock(now())) as Arc<dyn anima_hub::clock::Clock>)
        .finish()
}

pub fn signed_in(query: &str, variables: Value) -> Request {
    let operator = Operator {
        id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
    };
    Identity::for_tests(operator, now() + TimeDelta::days(3650))
        .attach(Request::new(query).variables(Variables::from_json(variables)))
        .data(ClientAddr(CLIENT_ADDR.parse().unwrap()))
}

pub async fn run(schema: &TestSchema, query: &str, variables: Value) -> Response {
    schema.execute(signed_in(query, variables)).await
}

/// The data of a signed-in request that must have no errors.
pub async fn data(schema: &TestSchema, query: &str, variables: Value) -> Value {
    let response = run(schema, query, variables).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

pub fn error_json(response: &Response) -> Value {
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    serde_json::to_value(&response.errors[0]).unwrap()
}

pub fn create_query() -> String {
    format!(
        "mutation($deviceId: ID!, $input: WebInterfaceInput!) \
         {{ createWebInterface(deviceId: $deviceId, input: $input) {{ {FIELDS} }} }}"
    )
}

pub fn update_query() -> String {
    format!(
        "mutation($id: ID!, $input: WebInterfaceInput!) \
         {{ updateWebInterface(id: $id, input: $input) {{ {FIELDS} }} }}"
    )
}

pub fn accept_certificate_query() -> String {
    format!(
        "mutation($id: ID!, $fingerprint: String!) \
         {{ acceptWebCertificate(id: $id, fingerprintSha256: $fingerprint) {{ {FIELDS} }} }}"
    )
}

pub fn accept_suggestion_query() -> String {
    format!(
        "mutation($checkId: ID!) {{ acceptWebInterfaceSuggestion(checkId: $checkId) {{ {FIELDS} }} }}"
    )
}

pub const DELETE: &str = "mutation($id: ID!) { deleteWebInterface(id: $id) }";
pub const DISMISS: &str =
    "mutation($checkId: ID!) { dismissWebInterfaceSuggestion(checkId: $checkId) }";

pub fn input(url: &str) -> Value {
    json!({ "label": "Panel", "url": url })
}

/// Creates an interface on `pve-01` and returns its `id`.
pub async fn create(schema: &TestSchema, url: &str) -> String {
    let created = data(
        schema,
        &create_query(),
        json!({ "deviceId": PVE_01, "input": input(url) }),
    )
    .await;
    created["createWebInterface"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}
