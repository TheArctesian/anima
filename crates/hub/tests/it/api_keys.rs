//! API keys through the hub's router: made by a signed-in operator, shown once, stored as a
//! SHA-256, refused on the D-23 list before any resolver, revoked at once and audited
//! (INT-003, LOG-005, LOG-007, D-23, P3-004).

mod create;
mod refused;
mod revoke;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start};

const CREATE: &str = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { \
    secret key { id name scope prefix expiresAt createdAt lastUsedAt } } }";

const REVOKE: &str = "mutation($id: ID!) { revokeApiKey(id: $id) }";

const LIST: &str = "{ apiKeys { id name scope prefix expiresAt createdAt lastUsedAt } }";

const ME: &str = "{ me { username } }";

/// A hub at `start()` with the fixture operator and a login token.
async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

/// Runs `createApiKey` with `token`; returns the whole response.
async fn create(hub: &Hub, token: &str, input: Value) -> Value {
    hub.post(Some(token), CREATE, json!({ "input": input }))
        .await
}

/// The input of a key named `name` with `scope` that expires in 30 days.
fn input(name: &str, scope: &str) -> Value {
    json!({ "name": name, "scope": scope, "expiresAt": start() + TimeDelta::days(30) })
}

/// A key made with `token`, as `createApiKey` answered it.
async fn made(hub: &Hub, token: &str, name: &str, scope: &str) -> Value {
    let response = create(hub, token, input(name, scope)).await;
    let created = &response["data"]["createApiKey"];
    assert!(created.is_object(), "{response}");
    created.clone()
}

/// The secret of a key made with `token`.
async fn secret(hub: &Hub, token: &str, name: &str, scope: &str) -> String {
    made(hub, token, name, scope).await["secret"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// The text after `anima_` in a key's secret.
fn body(secret: &str) -> &str {
    secret.strip_prefix("anima_").expect("an API key secret")
}

/// The hash the hub stores for a key's secret.
fn hash(secret: &str) -> Vec<u8> {
    Sha256::digest(URL_SAFE_NO_PAD.decode(body(secret)).unwrap()).to_vec()
}

/// The response's one error as `(code, fields)`.
fn coded(response: &Value) -> (String, Value) {
    let errors = response["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1, "{response}");
    let extensions = &errors[0]["extensions"];
    (
        extensions["code"].as_str().unwrap().to_owned(),
        extensions["fields"].clone(),
    )
}

/// Every row of `table` as JSON text.
async fn rows(pool: &PgPool, table: &str) -> Vec<String> {
    sqlx::query_scalar(&format!("select row_to_json(t)::text from {table} t"))
        .fetch_all(pool)
        .await
        .unwrap()
}

/// How many rows `table` holds.
async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("select count(*) from {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

fn instant(value: &Value) -> DateTime<Utc> {
    value.as_str().unwrap().parse().unwrap()
}
