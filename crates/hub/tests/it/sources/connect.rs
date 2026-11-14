//! `connectGithubApp`: the key is proved with a mint before anything is stored (SRC-003,
//! SRC-008, PRJ-018); a bad key, a refused key or an unreachable source stores nothing. The
//! connect flow's `establish` leaves the same rows, audit entry and event (PRJ-039).

use anima_hub::{
    engine::sources::{Connection, establish},
    fleet::events::{Actor, Bus, ClientAddr, Operator, Signal, Trail},
    ids::{OperatorId, SourceAccountId},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{CONNECT, TEST_KEY, input, kit, kit_with, set_projects, stub::Stub, unreachable};
use crate::hub::{error, start};

async fn stored(pool: &PgPool) -> (i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM sources.accounts), (SELECT count(*) FROM credentials.source_secrets)",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn src_003_connect_proves_the_key_then_stores_the_account_and_its_sealed_key(pool: PgPool) {
    let kit = kit(&pool).await;
    let (response, id) = kit.connect().await;
    let account = &response["data"]["connectGithubApp"];
    assert_eq!(account["sourceKey"], "github");
    assert_eq!(account["instanceKey"], "4242/77");
    assert_eq!(account["credentialStored"], true);
    assert_eq!(account["health"], "PAUSED", "Projects is off");
    assert_eq!(account["pausedBy"], "EXTENSION_OFF");
    assert_eq!(stored(&pool).await, (1, 1));
    let seen = kit.stub.with(|s| s.seen.clone());
    assert_eq!(
        seen,
        [(
            "POST".to_owned(),
            "/app/installations/77/access_tokens".to_owned(),
            true
        )],
        "one mint, signed in with the App token; no run while Projects is off"
    );
    let audit: (String, String, String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT action, target_type, target_id, actor_kind, token_kind FROM events.audit_log WHERE target_type = 'source_account'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let words = |s: &str| Some(s.to_owned());
    assert_eq!(
        audit,
        (
            "create".to_owned(),
            "source_account".to_owned(),
            id.to_string(),
            words("user"),
            words("login")
        )
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_018_the_proving_token_is_kept_only_while_a_consumer_is_on(pool: PgPool) {
    let kit = kit(&pool).await;
    let tokens = kit.reach.transport.tokens();
    let (_, off) = kit.connect().await;
    assert!(
        tokens.fresh(off, start()).is_none(),
        "Projects is off: no run will use it, so it is dropped"
    );

    set_projects(&pool, true).await;
    let (_, on) = kit.connect().await;
    assert_eq!(on, off, "the same installation");
    assert!(
        tokens.fresh(on, start()).is_some(),
        "Projects is on: the connect's run uses it"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn src_003_the_same_app_and_installation_again_keeps_the_account_and_replaces_the_key(
    pool: PgPool,
) {
    let kit = kit(&pool).await;
    let (_, first) = kit.connect().await;
    let secret: (uuid::Uuid, Vec<u8>) =
        sqlx::query_as("SELECT id, ciphertext FROM credentials.source_secrets")
            .fetch_one(&pool)
            .await
            .unwrap();
    let (_, second) = kit.connect().await;
    assert_eq!(first, second);
    let again: (uuid::Uuid, Vec<u8>) =
        sqlx::query_as("SELECT id, ciphertext FROM credentials.source_secrets")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(again.0, secret.0, "the same row");
    assert_ne!(again.1, secret.1, "sealed again");
    let actions: Vec<String> =
        sqlx::query_scalar("SELECT action FROM events.audit_log WHERE target_type = 'source_account' ORDER BY at, action")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(actions, ["create", "update"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_018_a_bad_key_is_validation_and_nothing_is_sent_or_stored(pool: PgPool) {
    let kit = kit(&pool).await;
    for (bad, field) in [
        (
            json!({ "input": { "label": "Mine", "appId": "42x", "installationId": "77", "privateKey": *TEST_KEY } }),
            "appId",
        ),
        (
            input("-----BEGIN RSA PRIVATE KEY-----\nAAAA\n-----END RSA PRIVATE KEY-----\n"),
            "privateKey",
        ),
        (
            json!({ "input": { "label": " ", "appId": "4242", "installationId": "77", "privateKey": *TEST_KEY } }),
            "label",
        ),
    ] {
        let response = kit.hub.post(Some(&kit.token), CONNECT, bad).await;
        assert_eq!(error(&response).1, "VALIDATION", "{response}");
        assert_eq!(
            response["errors"][0]["extensions"]["fields"][0]["field"],
            field
        );
    }
    assert_eq!(stored(&pool).await, (0, 0));
    assert_eq!(kit.stub.paths(), Vec::<String>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_018_a_key_the_source_refuses_is_validation_and_nothing_is_stored(pool: PgPool) {
    let kit = kit(&pool).await;
    kit.stub.with(|s| s.mint_status = 401);
    let response = kit
        .hub
        .post(Some(&kit.token), CONNECT, input(&TEST_KEY))
        .await;
    assert_eq!(error(&response).1, "VALIDATION", "{response}");
    assert_eq!(stored(&pool).await, (0, 0));
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_018_an_unreachable_source_is_unavailable_and_nothing_is_stored(pool: PgPool) {
    let stub = Stub::start(&pool).await;
    let kit = kit_with(&pool, stub, unreachable()).await;
    let response = kit
        .hub
        .post(Some(&kit.token), CONNECT, input(&TEST_KEY))
        .await;
    assert_eq!(error(&response).1, "UNAVAILABLE", "{response}");
    assert_eq!(stored(&pool).await, (0, 0));
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_an_api_key_is_forbidden_on_connect_and_remove(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    let create = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let key_input = json!({ "name": "writer", "scope": "WRITE", "madeFor": "USER",
        "expiresAt": start() + chrono::TimeDelta::days(30) });
    let made = kit
        .hub
        .post(Some(&kit.token), create, json!({ "input": key_input }))
        .await;
    let secret = made["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("no key: {made}"));
    let response = kit.hub.post(Some(secret), CONNECT, input(&TEST_KEY)).await;
    assert_eq!(error(&response).1, "FORBIDDEN", "{response}");
    let remove = "mutation($id: ID!) { removeSourceAccount(id: $id, deleteRecords: true) }";
    let response = kit
        .hub
        .post(Some(secret), remove, json!({ "id": id }))
        .await;
    assert_eq!(error(&response).1, "FORBIDDEN", "{response}");
    assert_eq!(stored(&pool).await, (1, 1), "nothing changed");
    let listed = kit
        .hub
        .post(Some(secret), "{ sourceAccounts { id } }", Value::Null)
        .await;
    assert!(listed["errors"].is_null(), "an API key may list: {listed}");
}

/// What one connect left behind for the account `id`: its row, its key, its audit entry (the
/// change without the instance key) and its event, minus ids, times and the client address.
async fn left_behind(pool: &PgPool, id: SourceAccountId) -> (Value, Value, Value, String) {
    let account: Value = sqlx::query_scalar(
        "SELECT jsonb_build_object('source', source, 'label', label, \
         'secrets', (SELECT count(*) FROM credentials.source_secrets s WHERE s.id = a.credential_id)) \
         FROM sources.accounts a WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    let audit: (Value, Value) = sqlx::query_as(
        "SELECT jsonb_build_object('action', action, 'type', target_type, 'operator', operator_id, \
         'name', operator_name, 'actor', actor_kind, 'token', token_kind), change - 'instanceKey' \
         FROM events.audit_log WHERE target_id = $1",
    )
    .bind(id.to_string())
    .fetch_one(pool)
    .await
    .unwrap();
    let event: String = sqlx::query_scalar(
        "SELECT severity || ' ' || source || ' ' || message FROM events.event_log \
         ORDER BY at DESC, id DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (account, audit.0, audit.1, event)
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_establish_leaves_what_connect_github_app_leaves(pool: PgPool) {
    let kit = kit(&pool).await;
    let mut feed = kit.reach.feed.subscribe();
    let (_, by_api) = kit.connect().await;
    assert_eq!(feed.recv().await.unwrap(), by_api, "published");
    let api = left_behind(&pool, by_api).await;

    let (id, name): (OperatorId, String) =
        sqlx::query_as("SELECT id, username FROM access.operators")
            .fetch_one(&pool)
            .await
            .unwrap();
    let operator = Operator {
        id,
        name,
        actor: Actor::for_tests(),
    };
    let bus = Bus::new(16);
    let mut events = bus.subscribe();
    let trail = Trail::new(
        &bus,
        (operator, ClientAddr("127.0.0.1".parse().unwrap())),
        start(),
    );
    let connection = Connection {
        label: "Mine".to_owned(),
        app_id: "4242".to_owned(),
        installation_id: "78".to_owned(),
        private_key: Zeroizing::new(TEST_KEY.clone()),
    };
    let established = establish(&kit.at(start()), trail, connection)
        .await
        .unwrap();
    assert!(established.created);
    assert!(!established.consumer_on, "Projects is off");
    assert_eq!(feed.recv().await.unwrap(), established.id, "published");
    let Ok(Signal::Event(event)) = events.recv().await else {
        panic!("no event on the bus");
    };
    assert_eq!(event.message, "Source account Mine connected.");
    assert_eq!(left_behind(&pool, established.id).await, api);
    assert_eq!(stored(&pool).await, (2, 2));
    let instance: String =
        sqlx::query_scalar("SELECT instance_key FROM sources.accounts WHERE id = $1")
            .bind(established.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(instance, "4242/78");
}
