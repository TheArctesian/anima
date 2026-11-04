//! The hub's HTTP edge serves the devices part: a write seals with the vault and audits the
//! operator and client address of the request (WP-06 H-2, D-6).

use serde_json::json;
use sqlx::PgPool;

use crate::hub::{CLIENT, Hub, USERNAME, set_operator, start};

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
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
