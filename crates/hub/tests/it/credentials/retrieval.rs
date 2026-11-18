//! R-147, K-E5 Section 2.1: a daemon fetches the operational keys granted to its device with its
//! token, reports what it holds with each push and hears the current versions; the three
//! refusals of a fetch are one answer, and a removed device gets nothing.

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::daemon::{
    batches::cpu_batch,
    harness::{Daemons, PERMITTED, PVE_01, pushing_now, refusal},
};

const VALUE: &str = "sk-live-daemon-fetch-Value";

const CREATE: &str = "mutation($input: CreateOperationalKeyInput!) { \
    createOperationalKey(input: $input) { name activeVersion } }";
const GRANT: &str = "mutation($name: String!, $device: ID!) { \
    grantOperationalKey(name: $name, device: $device, workload: \"ddns\") { name } }";
const ROTATE: &str = "mutation($name: String!, $value: String!) { \
    rotateOperationalKey(name: $name, value: $value, rotatedAtProvider: false) { endedVersion } }";
const REVOKE: &str =
    "mutation($name: String!) { revokeOperationalKey(name: $name) { endedVersion } }";
const SHOW: &str = "query($name: String!) { operationalKey(name: $name) { \
    consumers { deviceId heldVersion fetchedVersion } } }";

async fn create(daemons: &Daemons, name: &str) {
    let input = json!({ "name": name, "provider": "Cloudflare", "purpose": "DNS", "value": VALUE });
    let made = daemons.operate(CREATE, json!({ "input": input })).await;
    assert_eq!(
        made["data"]["createOperationalKey"]["activeVersion"], 1,
        "{made}"
    );
}

async fn grant(daemons: &Daemons, name: &str, device: &str) {
    let granted = daemons
        .operate(GRANT, json!({ "name": name, "device": device }))
        .await;
    assert_eq!(
        granted["data"]["grantOperationalKey"]["name"], name,
        "{granted}"
    );
}

/// `POST /daemon/v1/keys/fetch` of `name` for `device` with `token`.
async fn fetch(
    daemons: &Daemons,
    token: &str,
    (device, name): (&str, &str),
) -> (StatusCode, String) {
    let body = json!({ "device": device, "name": name }).to_string();
    let request = Request::post("/daemon/v1/keys/fetch")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body))
        .unwrap();
    daemons.send(request, PERMITTED).await
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_147_a_granted_daemon_fetches_the_active_version_and_no_older_one(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let token = daemons.token_for(PVE_01).await;
    create(&daemons, "cloudflare.dns").await;
    grant(&daemons, "cloudflare.dns", PVE_01).await;

    let (status, text) = fetch(&daemons, &token, (PVE_01, "cloudflare.dns")).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let fetched: Value = serde_json::from_str(&text).unwrap();
    let encoded = STANDARD.encode(VALUE);
    assert_eq!(
        fetched,
        json!({ "name": "cloudflare.dns", "version": 1, "value": encoded })
    );

    let rotated = daemons
        .operate(
            ROTATE,
            json!({ "name": "cloudflare.dns", "value": "second" }),
        )
        .await;
    assert_eq!(
        rotated["data"]["rotateOperationalKey"]["endedVersion"], 1,
        "{rotated}"
    );
    let (_, text) = fetch(&daemons, &token, (PVE_01, "cloudflare.dns")).await;
    let fetched: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(fetched["version"], 2);
    assert_eq!(fetched["value"], STANDARD.encode("second"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_147_no_grant_no_key_and_no_active_version_get_the_same_answer(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let token = daemons.token_for(PVE_01).await;
    create(&daemons, "ungranted").await;
    create(&daemons, "revoked").await;
    grant(&daemons, "revoked", PVE_01).await;
    daemons.operate(REVOKE, json!({ "name": "revoked" })).await;

    let answers = [
        fetch(&daemons, &token, (PVE_01, "ungranted")).await,
        fetch(&daemons, &token, (PVE_01, "missing")).await,
        fetch(&daemons, &token, (PVE_01, "revoked")).await,
    ];

    for answer in &answers {
        assert_eq!(*answer, (StatusCode::NOT_FOUND, refusal("NO_KEY")));
    }
    let refused = (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"));
    let junk = "anima_daemon_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    assert_eq!(fetch(&daemons, junk, (PVE_01, "revoked")).await, refused);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_147_a_removed_device_is_refused_and_its_grants_are_gone(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let token = daemons.token_for(PVE_01).await;
    create(&daemons, "cloudflare.dns").await;
    grant(&daemons, "cloudflare.dns", PVE_01).await;

    let deleted = daemons
        .operate(
            "mutation($id: ID!) { deleteDevice(id: $id) }",
            json!({ "id": PVE_01 }),
        )
        .await;
    assert_eq!(deleted["data"]["deleteDevice"], PVE_01, "{deleted}");

    let refused = (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"));
    assert_eq!(
        fetch(&daemons, &token, (PVE_01, "cloudflare.dns")).await,
        refused
    );
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM credentials.operational_key_consumers")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_147_a_push_that_says_what_it_holds_hears_the_current_versions(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    create(&daemons, "cloudflare.dns").await;
    grant(&daemons, "cloudflare.dns", PVE_01).await;

    let plain = cpu_batch(PVE_01, now, 10.0);
    assert_eq!(daemons.push(&token, &plain).await.0, StatusCode::NO_CONTENT);

    let mut with_keys = cpu_batch(PVE_01, now, 11.0);
    with_keys["keys"] = json!({ "held": [ { "name": "cloudflare.dns", "version": 1 } ] });
    let (status, text) = daemons.push(&token, &with_keys).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let accepted: Value = serde_json::from_str(&text).unwrap();
    let current = json!({ "keys": { "current": [ { "name": "cloudflare.dns", "version": 1 } ] } });
    assert_eq!(accepted, current);

    let shown = daemons
        .operate(SHOW, json!({ "name": "cloudflare.dns" }))
        .await;
    let consumer = &shown["data"]["operationalKey"]["consumers"][0];
    assert_eq!(consumer["heldVersion"], 1, "{shown}");
    assert_eq!(consumer["fetchedVersion"], Value::Null);

    let mut bad = cpu_batch(PVE_01, now, 12.0);
    bad["keys"] = json!({ "held": [ { "name": "Not A Name", "version": 1 } ] });
    let (status, text) = daemons.push(&token, &bad).await;
    assert_eq!(
        (status, text),
        (StatusCode::BAD_REQUEST, refusal("MALFORMED"))
    );
}
