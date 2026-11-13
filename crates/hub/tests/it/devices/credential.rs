//! `setSshCredential`, `clearSshCredential` and `SshSettings.credentialStored` (DEV-012,
//! SEC-003, SEC-005, LOG-005, LOG-007, DEV-015).

use std::fmt::Write as _;

use anima_hub::{
    credentials::{Secret, SshSecret, load_ssh, store_ssh},
    events::Signal,
    ids::DeviceId,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    harness::{error_json, schema_for, schema_with_bus, validation_fields, vault, with_variables},
    leak::{assert_absent, captured},
};
use crate::hub::host_of;

pub const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
/// PVE-01's host in the fixture, which its stored credentials are bound to (SEC-011).
pub const PVE_01_HOST: &str = "pve-01.home.lan";
const UNKNOWN: &str = "0190a000-0000-7000-8002-0000000000ff";
const PASSWORD: &str = "hunter2-correct-horse-battery";
const KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEthrowawaybody\n-----END OPENSSH PRIVATE KEY-----\n";
/// A part of `KEY` that only the key holds; the redaction hides whole strings with key markers,
/// so the body is what must never appear.
const KEY_BODY: &str = "b3BlbnNzaC1rZXktdjEthrowawaybody";
const PASSPHRASE: &str = "open-sesame-passphrase-42";

pub const SET: &str = "mutation($id: ID!, $credential: SshCredentialInput!) {
    setSshCredential(deviceId: $id, credential: $credential) { enabled username credentialStored } }";
const CLEAR: &str =
    "mutation($id: ID!) { clearSshCredential(deviceId: $id) { username credentialStored } }";
const STORED: &str = "query($id: ID!) { device(id: $id) { ssh { credentialStored } } }";

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

/// Stores a password for the device directly through `credentials`.
pub async fn store_password(pool: &PgPool, id: &str, password: &str) {
    let secret = SshSecret::Password(Secret::from(password.to_owned()));
    let mut conn = pool.acquire().await.unwrap();
    let host = host_of(pool, device(id)).await;
    store_ssh(&mut conn, &vault(), (device(id), &host), secret)
        .await
        .unwrap();
}

async fn secrets(pool: &PgPool, id: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM credentials.secrets WHERE device_id = $1")
        .bind(device(id))
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn audits(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar("SELECT to_jsonb(a) FROM events.audit_log a ORDER BY at, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn events(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar("SELECT to_jsonb(e) FROM events.event_log e ORDER BY at, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// Every row of every table, as text, so a test can search the whole database for a value.
async fn every_row(pool: &PgPool) -> String {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT format('%I.%I', table_schema, table_name) FROM information_schema.tables \
         WHERE table_schema NOT IN ('pg_catalog', 'information_schema') \
           AND table_type = 'BASE TABLE'",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let mut dump = String::new();
    for table in tables {
        let rows: Vec<String> =
            sqlx::query_scalar(&format!("SELECT to_jsonb(t)::text FROM {table} t"))
                .fetch_all(pool)
                .await
                .unwrap();
        dump.push_str(&rows.join("\n"));
    }
    dump
}

/// The secret as text and as the hex a `bytea` column shows.
fn forms(secret: &str) -> [String; 2] {
    let hex = secret.bytes().fold(String::new(), |mut hex, b| {
        write!(hex, "{b:02x}").unwrap();
        hex
    });
    [secret.to_owned(), hex]
}

fn private_key(passphrase: Option<&str>) -> Value {
    json!({ "privateKey": { "key": KEY, "passphrase": passphrase } })
}

fixture_test! {
    async fn log_007_a_stored_secret_is_in_no_log_no_response_no_bus_message_and_no_plaintext_column(pool: PgPool) {
        let cases = [
            (json!({ "password": PASSWORD }), vec![PASSWORD], json!({ "kind": "password" })),
            (private_key(None), vec![KEY_BODY], json!({ "kind": "privateKey", "encrypted": false })),
            (private_key(Some(PASSPHRASE)), vec![KEY_BODY, PASSPHRASE],
             json!({ "kind": "privateKey", "encrypted": true })),
        ];
        for (round, (credential, secrets_sent, change)) in cases.into_iter().enumerate() {
            let (schema, bus) = schema_with_bus(&pool);
            let mut receiver = bus.subscribe();
            let (log, response) = captured(|| {
                with_variables(&schema, SET, json!({ "id": PVE_01, "credential": credential }))
            })
            .await;
            assert!(response.errors.is_empty(), "{:?}", response.errors);
            let response = serde_json::to_string(&response).unwrap();
            assert!(response.contains(r#""credentialStored":true"#), "{response}");
            let Signal::Event(event) = receiver.try_recv().unwrap() else { panic!("an event") };
            assert_eq!(event.message, "Device pve-01 SSH credential set.");
            let audit = audits(&pool).await.pop().unwrap();
            assert_eq!(audit["change"], change, "round {round}");
            assert_eq!((audit["action"].clone(), audit["target_type"].clone()),
                (json!("update"), json!("credential")));
            assert_eq!(audit["target_id"], PVE_01);
            assert_eq!(events(&pool).await.len(), round + 1);
            let dump = every_row(&pool).await;
            for secret in secrets_sent {
                for form in forms(secret) {
                    assert_absent(&form, &[&dump, &response, &format!("{event:?}"), &log]);
                }
            }
        }
    }

    async fn sec_003_the_stored_secret_opens_with_the_hub_key_and_reads_back_whole(pool: PgPool) {
        let response = with_variables(&schema_for(&pool), SET,
            json!({ "id": PVE_01, "credential": private_key(Some(PASSPHRASE)) })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let Some(SshSecret::PrivateKey { key, passphrase: Some(passphrase) }) =
            load_ssh(&pool, &vault(), (device(PVE_01), PVE_01_HOST)).await.unwrap()
        else {
            panic!("a key with a passphrase");
        };
        assert_eq!((key.expose(), passphrase.expose()), (KEY.as_bytes(), PASSPHRASE.as_bytes()));
    }

    async fn dev_012_set_replaces_the_earlier_secret_and_credential_stored_follows(pool: PgPool) {
        let schema = schema_for(&pool);
        let read = || with_variables(&schema, STORED, json!({ "id": PVE_01 }));
        let stored = |response: async_graphql::Response| {
            response.data.into_json().unwrap()["device"]["ssh"]["credentialStored"].clone()
        };
        assert_eq!(stored(read().await), json!(false));
        store_password(&pool, PVE_01, "an-earlier-password").await;
        assert_eq!(stored(read().await), json!(true));
        let response = with_variables(&schema, SET,
            json!({ "id": PVE_01, "credential": { "password": PASSWORD } })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let Some(SshSecret::Password(password)) =
            load_ssh(&pool, &vault(), (device(PVE_01), PVE_01_HOST)).await.unwrap()
        else {
            panic!("a password");
        };
        assert_eq!(password.expose(), PASSWORD.as_bytes());
        assert_eq!(secrets(&pool, PVE_01).await, 1);
    }

    async fn log_005_a_failed_audit_keeps_the_earlier_secret_and_leaves_no_event(pool: PgPool) {
        store_password(&pool, PVE_01, "the-earlier-password").await;
        sqlx::query("DROP TABLE events.audit_log").execute(&pool).await.unwrap();
        let (schema, bus) = schema_with_bus(&pool);
        let mut receiver = bus.subscribe();
        for (query, variables) in [
            (SET, json!({ "id": PVE_01, "credential": { "password": PASSWORD } })),
            (CLEAR, json!({ "id": PVE_01 })),
        ] {
            let error = error_json(&with_variables(&schema, query, variables).await);
            assert_eq!(error["extensions"]["code"], "INTERNAL", "{error}");
        }
        let Some(SshSecret::Password(password)) =
            load_ssh(&pool, &vault(), (device(PVE_01), PVE_01_HOST)).await.unwrap()
        else {
            panic!("the earlier password");
        };
        assert_eq!(password.expose(), b"the-earlier-password");
        assert_eq!(events(&pool).await.len(), 0);
        assert!(receiver.try_recv().is_err());
    }

    async fn log_005_clear_removes_the_secret_and_audits_once_and_a_second_clear_writes_nothing(pool: PgPool) {
        store_password(&pool, PVE_01, PASSWORD).await;
        let schema = schema_for(&pool);
        for _ in 0..2 {
            let response = with_variables(&schema, CLEAR, json!({ "id": PVE_01 })).await;
            assert!(response.errors.is_empty(), "{:?}", response.errors);
            assert_eq!(response.data.into_json().unwrap(), json!({ "clearSshCredential":
                { "username": "operator", "credentialStored": false } }));
        }
        assert_eq!(secrets(&pool, PVE_01).await, 0);
        let (audits, events) = (audits(&pool).await, events(&pool).await);
        assert_eq!((audits.len(), events.len()), (1, 1));
        assert_eq!((audits[0]["action"].clone(), audits[0]["target_type"].clone()),
            (json!("delete"), json!("credential")));
        assert_eq!(audits[0]["change"], json!({ "cleared": true }));
        assert_eq!(events[0]["message"], "Device pve-01 SSH credential cleared.");
    }

    async fn dev_012_an_unknown_device_is_not_found_and_a_bad_id_is_a_field_error(pool: PgPool) {
        let schema = schema_for(&pool);
        let password = json!({ "password": PASSWORD });
        for (query, variables) in [
            (SET, json!({ "id": UNKNOWN, "credential": password })),
            (CLEAR, json!({ "id": UNKNOWN })),
        ] {
            let error = error_json(&with_variables(&schema, query, variables).await);
            assert_eq!(error["extensions"]["code"], "NOT_FOUND", "{error}");
        }
        let error = error_json(&with_variables(&schema, SET,
            json!({ "id": "pve-01", "credential": password })).await);
        assert_eq!(validation_fields(&error), ["deviceId"]);
        let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM credentials.secrets")
            .fetch_one(&pool).await.unwrap();
        assert_eq!((stored, audits(&pool).await.len()), (0, 0));
    }

    async fn dev_012_bad_credentials_are_field_errors_that_quote_no_value(pool: PgPool) {
        let long_key = format!("{KEY}{}", "A".repeat(16 * 1024));
        let long_text = "p".repeat(1025);
        let key = |key: &str, passphrase: Option<&str>| {
            json!({ "privateKey": { "key": key, "passphrase": passphrase } })
        };
        let cases = [
            (json!({ "password": "" }), vec!["credential.password"]),
            (json!({ "password": " \t\n " }), vec!["credential.password"]),
            (json!({ "password": long_text }), vec!["credential.password"]),
            (key("", None), vec!["credential.privateKey.key"]),
            (key("   ", Some(" ")), vec!["credential.privateKey.key", "credential.privateKey.passphrase"]),
            (key("ssh-ed25519 AAAAC3Nza-not-a-private-key", None), vec!["credential.privateKey.key"]),
            (key("-----BEGIN PUBLIC KEY-----\nMIIBIjAN\n-----END PUBLIC KEY-----", None),
             vec!["credential.privateKey.key"]),
            (key("PRIVATE KEY----- -----BEGIN ", None), vec!["credential.privateKey.key"]),
            (key("-----BEGIN PUBLIC KEY-----\nMIIBIjAN\n-----END PUBLIC KEY-----\n-----BEGIN RSA PRIVATE KEY-----", None),
             vec!["credential.privateKey.key"]),
            (key("-----BEGIN PUBLIC KEY----- PRIVATE KEY-----\nMIIBIjAN", None),
             vec!["credential.privateKey.key"]),
            (key(&long_key, None), vec!["credential.privateKey.key"]),
            (key(KEY, Some(&long_text)), vec!["credential.privateKey.passphrase"]),
            (key(KEY, Some("")), vec!["credential.privateKey.passphrase"]),
        ];
        let schema = schema_for(&pool);
        for (credential, expected) in cases {
            let sent = credential.to_string();
            let response = with_variables(&schema, SET,
                json!({ "id": PVE_01, "credential": credential })).await;
            let error = error_json(&response);
            assert_eq!(validation_fields(&error), expected, "{sent:.80}");
            let text = error.to_string();
            let key_length = long_key.len().to_string();
            for quoted in ["AAAAC3Nza", "MIIBIjAN", "pppppppp", "AAAAAAAA", KEY_BODY, "1025", &key_length] {
                assert!(!text.contains(quoted), "{quoted} in {text}");
            }
        }
        assert_eq!((secrets(&pool, PVE_01).await, audits(&pool).await.len()), (0, 0));
    }

    async fn dev_015_deleting_a_device_removes_its_secret(pool: PgPool) {
        store_password(&pool, PVE_01, PASSWORD).await;
        let response = with_variables(&schema_for(&pool),
            "mutation($id: ID!) { deleteDevice(id: $id) }", json!({ "id": PVE_01 })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert_eq!(secrets(&pool, PVE_01).await, 0);
    }
}
