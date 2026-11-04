//! The hub as `schema.rs` and `lib.rs` wire it serves the devices part: every loader, the
//! vault, the bus, and the operator and client address of each request (WP-06 H-2).

use serde_json::json;
use sqlx::PgPool;

use crate::hub::{CLIENT, Hub, USERNAME, set_operator, start};

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn h_2_a_signed_in_read_reaches_every_devices_loader(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let query = format!(
        r#"{{ device(id: "{PVE_01}") {{ name site {{ name devices(first: 1) {{ totalCount }} }}
            ssh {{ credentialStored }} }} }}"#
    );

    let response = hub.post(Some(&token), &query, json!({})).await;

    assert!(response.get("errors").is_none(), "{response}");
    let device = &response["data"]["device"];
    assert_eq!(device["name"], "pve-01", "{response}");
    assert!(device["site"]["name"].is_string(), "{response}");
    assert!(device["site"]["devices"]["totalCount"].as_i64().unwrap() >= 1);
    assert_eq!(device["ssh"]["credentialStored"], false, "{response}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn h_2_a_write_seals_with_the_vault_and_audits_the_operator_and_client(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let mut events = hub.bus.subscribe();
    let set = "mutation($id: ID!, $c: SshCredentialInput!) { \
               setSshCredential(deviceId: $id, credential: $c) { credentialStored } }";
    let variables = json!({ "id": PVE_01, "c": { "password": "a long ssh password" } });

    let response = hub.post(Some(&token), set, variables).await;

    assert_eq!(
        response,
        json!({ "data": { "setSshCredential": { "credentialStored": true } } })
    );
    let (operator, client): (String, String) = sqlx::query_as(
        "select operator_name, host(client_addr) from events.audit_log \
         where target_id = $1",
    )
    .bind(PVE_01)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((operator.as_str(), client.as_str()), (USERNAME, CLIENT));
    assert!(events.try_recv().is_ok(), "the event reached the hub's bus");
}
