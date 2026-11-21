//! The catalogue through the hub's router (task H38, R-234): the list and the description are
//! open to each token, the install is open to a write key and an agent, and the install is
//! refused with a fixed reason. No request reaches a machine.

use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start};

const LIST: &str = "{ catalogueEntries { id kind lastMachineTest install { refusal } } }";
const SHOW: &str = "query($id: String!) { catalogueEntry(id: $id) { id composeFile \
    environmentVariables install { refusal reason needsOpenToNetwork } } }";
const INSTALL: &str = "mutation($entry: String!, $device: ID!) { \
    installCatalogueEntry(entryId: $entry, deviceId: $device) { refusal reason } }";
const CREATE_KEY: &str =
    "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
const DEVICE: &str = "0190a000-0000-7000-8002-000000000002";

async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

async fn key(hub: &Hub, token: &str, scope: &str) -> String {
    let input = json!({ "name": format!("agent-{scope}"), "scope": scope, "madeFor": "AGENT",
        "expiresAt": start() + chrono::TimeDelta::days(30) });
    let made = hub
        .post(Some(token), CREATE_KEY, json!({ "input": input }))
        .await;
    made["data"]["createApiKey"]["secret"]
        .as_str()
        .expect("a key")
        .to_owned()
}

fn install(entry: &str, device: &str) -> Value {
    json!({ "entry": entry, "device": device })
}

#[sqlx::test(migrations = "./migrations")]
async fn h38_the_hub_lists_and_describes_the_entries_that_pass_the_static_test(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let answer = hub.post(Some(&token), LIST, json!({})).await;
    let entries = answer["data"]["catalogueEntries"]
        .as_array()
        .expect("a list");
    let ids: Vec<&str> = entries
        .iter()
        .filter_map(|entry| entry["id"].as_str())
        .collect();
    assert!(ids.contains(&"postgresql"), "{answer}");
    assert!(!ids.contains(&"pihole"), "{answer}");
    for entry in entries {
        assert_eq!(entry["lastMachineTest"], Value::Null, "{entry}");
        assert_eq!(entry["install"]["refusal"], "INSTALL_NOT_BUILT", "{entry}");
    }

    let answer = hub
        .post(Some(&token), SHOW, json!({ "id": "postgresql" }))
        .await;
    let entry = &answer["data"]["catalogueEntry"];
    assert_eq!(entry["environmentVariables"], json!(["POSTGRES_PASSWORD"]));
    assert!(entry["composeFile"].as_str().unwrap().contains("expose:"));
    assert!(
        entry["install"]["reason"]
            .as_str()
            .unwrap()
            .contains("task H6")
    );
    let answer = hub
        .post(Some(&token), SHOW, json!({ "id": "pihole" }))
        .await;
    assert_eq!(answer["data"]["catalogueEntry"], Value::Null, "{answer}");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_234_a_write_key_asks_for_an_install_and_gets_the_fixed_refusal(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let write = key(&hub, &token, "WRITE").await;
    let answer = hub
        .post(Some(&write), INSTALL, install("postgresql", DEVICE))
        .await;
    let refused = &answer["data"]["installCatalogueEntry"];
    assert_eq!(refused["refusal"], "INSTALL_NOT_BUILT", "{answer}");
    let answer = hub
        .post(Some(&write), INSTALL, install("pihole", DEVICE))
        .await;
    assert_eq!(
        answer["data"]["installCatalogueEntry"]["refusal"],
        "NOT_LISTED"
    );
    let answer = hub
        .post(Some(&token), INSTALL, install("postgresql", "not-an-id"))
        .await;
    assert_eq!(
        answer["errors"][0]["extensions"]["code"], "VALIDATION",
        "{answer}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn h38_a_read_key_lists_but_does_not_ask_for_an_install(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let read = key(&hub, &token, "READ").await;
    let answer = hub.post(Some(&read), LIST, json!({})).await;
    assert!(answer["data"]["catalogueEntries"].is_array(), "{answer}");
    let answer = hub
        .post(Some(&read), INSTALL, install("postgresql", DEVICE))
        .await;
    assert_eq!(answer["data"], Value::Null, "{answer}");
    assert_eq!(answer["errors"][0]["extensions"]["code"], "FORBIDDEN");
    let answer = hub.post(None, LIST, json!({})).await;
    assert_eq!(answer["errors"][0]["extensions"]["code"], "UNAUTHENTICATED");
}
