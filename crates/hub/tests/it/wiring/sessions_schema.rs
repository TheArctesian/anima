//! WP-10 handoffs 1 and 2: the sessions part of the hub schema runs on the `Live` that
//! `Dependencies` carries, so a ticket the schema issues is one that `Live` redeems.

use anima_hub::sessions::Presented;
use anima_hub::{events::ClientAddr, sessions::Live};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    CLIENT_ADDR, FINGERPRINT, PVE_01, UNKNOWN_DEVICE, accept_key, add_operator, audit_clients,
    data, error_code, hub_schema, now, session_end,
};

const START: &str = "mutation($input: StartSessionInput!) { startSession(input: $input) { \
                     sessionId token tunnelPath expiresAt } }";

/// Starts a session on PVE-01 through `schema` and returns the ticket.
async fn start_on_pve_01(schema: &anima_hub::schema::HubSchema) -> Value {
    let input = json!({ "deviceId": PVE_01, "protocol": "SSH",
                        "display": { "widthPx": 1280, "heightPx": 800 } });
    data(schema, START, json!({ "input": input })).await["startSession"].clone()
}

async fn ready(pool: &PgPool) {
    add_operator(pool).await;
    accept_key(pool, PVE_01, "operator").await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_032_start_session_returns_a_ticket_the_hubs_one_live_redeems(pool: PgPool) {
    ready(&pool).await;
    let live = Live::default();
    let schema = hub_schema(&pool, &live);

    let ticket = start_on_pve_01(&schema).await;

    let token = ticket["token"].as_str().unwrap();
    let session = ticket["sessionId"].as_str().unwrap();
    assert!(!token.is_empty(), "{ticket}");
    assert!(
        ticket["tunnelPath"]
            .as_str()
            .unwrap()
            .starts_with("/terminal")
    );
    let client = ClientAddr(CLIENT_ADDR.parse().unwrap());
    let redeemed = live.redeem(Presented::terminal(token, client), now());
    assert!(
        redeemed.is_ok(),
        "the schema's Live is the one Dependencies carries"
    );
    assert_eq!(session_end(&pool, session).await, (false, None));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_end_session_ends_the_started_session(pool: PgPool) {
    ready(&pool).await;
    let schema = hub_schema(&pool, &Live::default());
    let ticket = start_on_pve_01(&schema).await;
    let session = ticket["sessionId"].as_str().unwrap();

    let ended = data(
        &schema,
        "mutation($id: ID!) { endSession(id: $id) { id endReason endedAt recording } }",
        json!({ "id": session }),
    )
    .await;

    assert_eq!(ended["endSession"]["id"], session);
    assert_eq!(ended["endSession"]["endReason"], "OPERATOR");
    // Resolves only when the schema carries the `Session.recording` loader (RA-040, G-5).
    assert_eq!(ended["endSession"]["recording"], "OFF");
    assert_eq!(
        session_end(&pool, session).await,
        (true, Some("operator".to_owned()))
    );
    assert_eq!(
        audit_clients(&pool, session, "end").await,
        vec![CLIENT_ADDR]
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_a_devices_host_key_resolves_through_the_hub_schema(pool: PgPool) {
    ready(&pool).await;
    let schema = hub_schema(&pool, &Live::default());
    let result = data(
        &schema,
        "query($id: ID!) { device(id: $id) { ssh { hostKey { algorithm fingerprintSha256 } } } }",
        json!({ "id": PVE_01 }),
    )
    .await;
    assert_eq!(
        result["device"]["ssh"]["hostKey"],
        json!({ "algorithm": "ssh-ed25519", "fingerprintSha256": FINGERPRINT })
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_013_test_connection_and_accept_host_key_are_reachable(pool: PgPool) {
    add_operator(&pool).await;
    let schema = hub_schema(&pool, &Live::default());
    let test = "mutation($input: ConnectionTestInput!) { testConnection(input: $input) { \
                reachable } }";
    let accept = "mutation($id: ID!, $fp: String!) { acceptHostKey(deviceId: $id, \
                  fingerprintSha256: $fp) { fingerprintSha256 } }";

    let tested = error_code(
        &schema,
        test,
        json!({ "input": { "deviceId": UNKNOWN_DEVICE } }),
    )
    .await;
    let accepted = error_code(
        &schema,
        accept,
        json!({ "id": UNKNOWN_DEVICE, "fp": FINGERPRINT }),
    )
    .await;

    assert_eq!(
        (tested.as_str(), accepted.as_str()),
        ("NOT_FOUND", "NOT_FOUND")
    );
}
