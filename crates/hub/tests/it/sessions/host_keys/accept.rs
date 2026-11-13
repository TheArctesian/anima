//! `acceptHostKey` (D-6, SEC-006, LOG-005): only the key a test saw in the last 15 minutes,
//! by its fingerprint; stored with an audit entry in the same transaction.

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::events::Bus,
    schema::{Dependencies, build_schema},
};
use chrono::{DateTime, TimeDelta, Utc};
use russh::keys::PrivateKey;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    accept_directly, accept_request, fingerprint, loopback_live, password, point_at, schema,
    server::{PASSWORD, SshServer, ed25519},
    store_credential, test_request, tested,
};
use crate::sessions::{
    OPERATOR_ID, PVE_01, add_operator,
    harness::{error_json, schema_at},
    now,
};

/// The stored host key row of PVE-01, as text.
#[derive(Debug, PartialEq, sqlx::FromRow)]
struct Row {
    algorithm: String,
    fingerprint_sha256: String,
    public_key: String,
    first_seen_at: DateTime<Utc>,
    accepted_at: DateTime<Utc>,
    accepted_by: String,
}

async fn row(pool: &PgPool) -> Option<Row> {
    sqlx::query_as(
        "select algorithm, fingerprint_sha256, public_key, first_seen_at, accepted_at, \
                accepted_by::text from trust.host_keys where device_id = $1::text::uuid",
    )
    .bind(PVE_01)
    .fetch_optional(pool)
    .await
    .unwrap()
}

/// The audit entries on host keys: action, target id and change, oldest first.
async fn host_key_audits(pool: &PgPool) -> Vec<(String, String, Value)> {
    sqlx::query_as(
        "select action, target_id, change from events.audit_log \
         where target_type = 'host_key' order by at",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

fn accept_entry(change: Value) -> (String, String, Value) {
    ("accept".to_owned(), PVE_01.to_owned(), change)
}

fn code(response: &async_graphql::Response) -> Value {
    error_json(response)["extensions"]["code"].clone()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_006_accept_without_a_test_or_with_another_fingerprint_is_conflict(pool: PgPool) {
    add_operator(&pool).await;
    let (presented, other) = (ed25519(1), ed25519(2));
    let server = SshServer::start(vec![presented.clone()], None).await;
    point_at(&pool, &server).await;
    let live = loopback_live();
    let schema = schema(&pool, &live);

    let unseen = schema
        .execute(accept_request(PVE_01, &fingerprint(&presented)))
        .await;
    tested(&schema, &json!({ "deviceId": PVE_01 })).await;
    let wrong = schema
        .execute(accept_request(PVE_01, &fingerprint(&other)))
        .await;
    let malformed = schema.execute(accept_request(PVE_01, "SHA256:short")).await;
    let later = schema_at(&pool, now() + TimeDelta::minutes(15), &live);
    let expired = later
        .execute(accept_request(PVE_01, &fingerprint(&presented)))
        .await;

    assert_eq!(code(&unseen), "CONFLICT");
    assert_eq!(code(&wrong), "CONFLICT");
    assert_eq!(code(&expired), "CONFLICT");
    assert_eq!(code(&malformed), "VALIDATION");
    assert_eq!(
        error_json(&malformed)["extensions"]["fields"][0]["field"],
        "fingerprintSha256"
    );
    assert_eq!(row(&pool).await, None);
    assert_eq!(host_key_audits(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_006_log_005_accepting_the_seen_key_stores_it_with_an_audit_entry(pool: PgPool) {
    add_operator(&pool).await;
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    point_at(&pool, &server).await;
    store_credential(&pool, password(PASSWORD)).await;
    let live = loopback_live();
    let schema = schema(&pool, &live);
    tested(&schema, &json!({ "deviceId": PVE_01 })).await;

    // Accepted a minute after the test: first seen stays the test's time.
    let later = schema_at(&pool, now() + TimeDelta::minutes(1), &live);
    let response = later
        .execute(accept_request(PVE_01, &fingerprint(&host_key)))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let accepted = response.data.into_json().unwrap()["acceptHostKey"].clone();
    assert_eq!(accepted["algorithm"], "ssh-ed25519");
    assert_eq!(accepted["fingerprintSha256"], fingerprint(&host_key));
    let openssh = host_key.public_key().to_openssh().unwrap();
    assert_eq!(
        row(&pool).await,
        Some(Row {
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint_sha256: fingerprint(&host_key),
            public_key: openssh.split(' ').nth(1).unwrap().to_owned(),
            first_seen_at: now(),
            accepted_at: now() + TimeDelta::minutes(1),
            accepted_by: OPERATOR_ID.to_owned(),
        })
    );
    let change = json!({ "deviceName": "pve-01", "algorithm": "ssh-ed25519",
        "oldFingerprint": null, "newFingerprint": fingerprint(&host_key) });
    assert_eq!(host_key_audits(&pool).await, [accept_entry(change)]);
    // The key is now the accepted one, so the next test signs in.
    let result = tested(&schema, &json!({ "deviceId": PVE_01 })).await;
    assert_eq!(result["authenticated"], true);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_replacing_a_key_records_both_fingerprints_and_the_same_key_again_writes_nothing(
    pool: PgPool,
) {
    let (old, new) = (ed25519(1), ed25519(2));
    let server = SshServer::start(vec![new.clone()], None).await;
    point_at(&pool, &server).await;
    accept_directly(&pool, &old).await;
    let schema = schema(&pool, &loopback_live());
    let tested_result = tested(&schema, &json!({ "deviceId": PVE_01 })).await;
    assert_eq!(tested_result["hostKeyChanged"], true);

    for _ in 0..2 {
        let response = schema
            .execute(accept_request(PVE_01, &fingerprint(&new)))
            .await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
    }

    let change = json!({ "deviceName": "pve-01", "algorithm": "ssh-ed25519",
        "oldFingerprint": fingerprint(&old), "newFingerprint": fingerprint(&new) });
    assert_eq!(
        host_key_audits(&pool).await,
        [accept_entry(change)],
        "one entry: the second accept changed nothing"
    );
    assert_eq!(
        row(&pool).await.unwrap().fingerprint_sha256,
        fingerprint(&new)
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_a_key_seen_for_unsaved_values_is_accepted_once_the_device_has_that_host(
    pool: PgPool,
) {
    add_operator(&pool).await;
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    let schema = schema(&pool, &loopback_live());
    let form = json!({ "host": "127.0.0.1", "ssh": { "enabled": true, "port": server.port,
        "username": "operator", "auth": "PASSWORD" } });
    tested(&schema, &form).await;
    // The operator saves the device with the tested values, then accepts.
    point_at(&pool, &server).await;

    let response = schema
        .execute(accept_request(PVE_01, &fingerprint(&host_key)))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert!(row(&pool).await.is_some());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_006_accept_for_an_unknown_device_is_not_found(pool: PgPool) {
    let schema = schema(&pool, &loopback_live());
    let response = schema
        .execute(accept_request(
            "0190a000-0000-7000-8002-0000000000ff",
            &fingerprint(&ed25519(1)),
        ))
        .await;
    assert_eq!(code(&response), "NOT_FOUND");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn d_31_the_hub_schema_accepts_the_key_its_connection_test_saw(pool: PgPool) {
    // `build_schema` as `main.rs` calls it: `testConnection` records into the `Live`'s
    // sightings and `acceptHostKey` reads the trust domain's copy of them (WP-41 T6).
    add_operator(&pool).await;
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    point_at(&pool, &server).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let schema = build_schema(Dependencies {
        live: loopback_live(),
        ..crate::hub::dependencies(&pool, clock, Bus::new(16))
    });
    let input = json!({ "deviceId": PVE_01 });

    let tested = schema.execute(test_request(&input)).await;
    let accepted = schema
        .execute(accept_request(PVE_01, &fingerprint(&host_key)))
        .await;

    assert!(tested.errors.is_empty(), "{:?}", tested.errors);
    assert!(accepted.errors.is_empty(), "{:?}", accepted.errors);
    assert_eq!(
        row(&pool).await.map(|row| row.fingerprint_sha256),
        Some(fingerprint(&host_key))
    );
}

/// Puts `key` back as PVE-01's accepted key, keeping the row's other columns.
async fn restore(pool: &PgPool, key: &PrivateKey) {
    let openssh = key.public_key().to_openssh().unwrap();
    sqlx::query(
        "update trust.host_keys set fingerprint_sha256 = $2, public_key = $3 \
         where device_id = $1::text::uuid",
    )
    .bind(PVE_01)
    .bind(fingerprint(key))
    .bind(openssh.split(' ').nth(1).unwrap())
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn log_005_two_accepts_at_once_audit_one_replacement_naming_the_key_it_replaced(
    pool: PgPool,
) {
    let (old, new) = (ed25519(1), ed25519(2));
    let server = SshServer::start(vec![new.clone()], None).await;
    point_at(&pool, &server).await;
    accept_directly(&pool, &old).await;
    let schema = schema(&pool, &loopback_live());
    tested(&schema, &json!({ "deviceId": PVE_01 })).await;

    for round in 1..=20 {
        restore(&pool, &old).await;
        let (first, second) = tokio::join!(
            schema.execute(accept_request(PVE_01, &fingerprint(&new))),
            schema.execute(accept_request(PVE_01, &fingerprint(&new))),
        );
        assert!(first.errors.is_empty(), "{:?}", first.errors);
        assert!(second.errors.is_empty(), "{:?}", second.errors);
        // One replacement per round: the second accept finds the new key and writes nothing.
        let audits = host_key_audits(&pool).await;
        assert_eq!(audits.len(), round, "round {round}");
        for (_, _, change) in &audits {
            assert_eq!(
                change["oldFingerprint"],
                json!(fingerprint(&old)),
                "round {round}"
            );
        }
    }
}
