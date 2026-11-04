//! `testConnection` (DEV-013, D-5): a first-seen or changed key is shown and no credential
//! leaves the hub; the accepted key signs in with a password or a private key.

use anima_hub::{
    credentials::{Secret, SshSecret},
    sessions::Live,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    accept_directly, fingerprint, loopback_live, password, point_at, schema,
    server::{Attempt, PASSWORD, SshServer, USER, ecdsa_p256, ed25519, openssh_text},
    store_credential, test_request, tested,
};
use crate::sessions::{PVE_01, harness::error_json};

fn saved() -> Value {
    json!({ "deviceId": PVE_01 })
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_sec_006_a_first_seen_key_is_shown_and_the_server_gets_no_auth_attempt(
    pool: PgPool,
) {
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    point_at(&pool, &server).await;
    store_credential(&pool, password(PASSWORD)).await;
    let schema = schema(&pool, &loopback_live());

    let stored = tested(&schema, &saved()).await;
    let given = json!({ "deviceId": PVE_01, "credential": { "password": PASSWORD } });
    let with_credential = tested(&schema, &given).await;

    for result in [&stored, &with_credential] {
        assert_eq!(result["reachable"], true);
        assert_eq!(result["authenticated"], false);
        assert_eq!(result["hostKeyChanged"], false);
        assert_eq!(result["hostKey"]["algorithm"], "ssh-ed25519");
        assert_eq!(
            result["hostKey"]["fingerprintSha256"],
            fingerprint(&host_key)
        );
        assert!(result["message"].as_str().unwrap().contains("accept"));
    }
    assert_eq!(
        server.attempts(),
        [],
        "no credential reached an unaccepted key"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_the_accepted_key_signs_in_with_the_stored_password(pool: PgPool) {
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    point_at(&pool, &server).await;
    accept_directly(&pool, &host_key).await;
    store_credential(&pool, password(PASSWORD)).await;

    let result = tested(&schema(&pool, &loopback_live()), &saved()).await;

    assert_eq!(
        result,
        json!({ "reachable": true, "authenticated": true, "hostKey": null,
                "hostKeyChanged": false, "message": "Connected and signed in." })
    );
    let expected = Attempt::Password {
        user: USER.to_owned(),
        correct: true,
    };
    assert_eq!(server.attempts(), [expected]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_the_accepted_key_signs_in_with_a_private_key_stored_or_given(pool: PgPool) {
    let (host_key, client_key) = (ed25519(1), ed25519(9));
    let server = SshServer::start(
        vec![host_key.clone()],
        Some(client_key.public_key().clone()),
    )
    .await;
    point_at(&pool, &server).await;
    accept_directly(&pool, &host_key).await;
    let schema = schema(&pool, &loopback_live());
    let stored = SshSecret::PrivateKey {
        key: Secret::from(openssh_text(&client_key, None)),
        passphrase: None,
    };
    store_credential(&pool, stored).await;

    let with_stored = tested(&schema, &saved()).await;
    let encrypted = openssh_text(&client_key, Some("a passphrase"));
    let given = json!({ "deviceId": PVE_01, "credential": { "privateKey": {
        "key": encrypted, "passphrase": "a passphrase" } } });
    let with_given = tested(&schema, &given).await;
    let wrong = json!({ "deviceId": PVE_01, "credential": { "privateKey": {
        "key": encrypted, "passphrase": "not the passphrase" } } });
    let with_wrong_passphrase = tested(&schema, &wrong).await;

    assert_eq!(with_stored["authenticated"], true);
    assert_eq!(with_given["authenticated"], true);
    assert_eq!(with_wrong_passphrase["authenticated"], false);
    assert_eq!(with_wrong_passphrase["reachable"], true);
    let key_attempts = server
        .attempts()
        .into_iter()
        .filter(|attempt| matches!(attempt, Attempt::PublicKey { .. }))
        .count();
    assert_eq!(key_attempts, 2, "an unreadable key is never offered");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_a_refused_password_is_a_result_with_a_fixed_message(pool: PgPool) {
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    point_at(&pool, &server).await;
    accept_directly(&pool, &host_key).await;
    store_credential(&pool, password("hunter2-wrong-password")).await;

    let result = tested(&schema(&pool, &loopback_live()), &saved()).await;

    assert_eq!(
        result,
        json!({ "reachable": true, "authenticated": false, "hostKey": null,
                "hostKeyChanged": false, "message": "The device refused the credential." })
    );
    assert!(!result.to_string().contains("hunter2"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_a_changed_key_is_flagged_and_no_credential_is_sent(pool: PgPool) {
    let (accepted, presented) = (ed25519(1), ed25519(2));
    let server = SshServer::start(vec![presented.clone()], None).await;
    point_at(&pool, &server).await;
    accept_directly(&pool, &accepted).await;
    store_credential(&pool, password(PASSWORD)).await;

    let result = tested(&schema(&pool, &loopback_live()), &saved()).await;

    assert_eq!(result["reachable"], true);
    assert_eq!(result["authenticated"], false);
    assert_eq!(result["hostKeyChanged"], true);
    assert_eq!(
        result["hostKey"]["fingerprintSha256"],
        fingerprint(&presented)
    );
    assert!(
        result["message"]
            .as_str()
            .unwrap()
            .contains("No credential was sent")
    );
    assert_eq!(server.attempts(), []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_the_probe_reads_the_key_type_libssh2_would_choose(pool: PgPool) {
    // guacd's libssh2 prefers ECDSA to ed25519, so the hub must read and pin the ECDSA key.
    let (ed25519_key, ecdsa_key) = (ed25519(1), ecdsa_p256());
    let server = SshServer::start(vec![ed25519_key, ecdsa_key.clone()], None).await;
    point_at(&pool, &server).await;

    let result = tested(&schema(&pool, &loopback_live()), &saved()).await;

    assert_eq!(result["hostKey"]["algorithm"], "ecdsa-sha2-nistp256");
    assert_eq!(
        result["hostKey"]["fingerprintSha256"],
        fingerprint(&ecdsa_key)
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_unsaved_values_are_tested_without_sending_a_credential(pool: PgPool) {
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    let schema = schema(&pool, &loopback_live());
    let input = json!({ "host": "127.0.0.1",
        "ssh": { "enabled": true, "port": server.port, "username": USER, "auth": "PASSWORD" },
        "credential": { "password": PASSWORD } });

    let result = tested(&schema, &input).await;

    assert_eq!(result["authenticated"], false);
    assert_eq!(
        result["hostKey"]["fingerprintSha256"],
        fingerprint(&host_key)
    );
    assert_eq!(server.attempts(), []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_a_closed_port_is_unreachable_not_an_error(pool: PgPool) {
    let server = SshServer::start(vec![ed25519(1)], None).await;
    point_at(&pool, &server).await;
    drop(server);
    tokio::task::yield_now().await;

    let result = tested(&schema(&pool, &loopback_live()), &saved()).await;

    assert_eq!(result["reachable"], false);
    assert_eq!(result["authenticated"], false);
    assert_eq!(result["hostKey"], Value::Null);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn r_032_the_hub_refuses_to_test_a_host_that_is_the_gateway_itself(pool: PgPool) {
    // The listening server proves the refusal is the rule, not a closed port.
    let server = SshServer::start(vec![ed25519(1)], None).await;
    let schema = schema(&pool, &Live::default());
    for host in ["localhost", "127.0.0.1", "::ffff:127.0.0.1", "::1"] {
        let input = json!({ "host": host, "ssh": { "enabled": true, "port": server.port,
            "username": USER, "auth": "PASSWORD" } });
        let error = error_json(&schema.execute(test_request(&input)).await);
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{host}");
        assert_eq!(error["extensions"]["fields"][0]["field"], "host", "{host}");
    }
    point_at(&pool, &server).await;
    let error = error_json(&schema.execute(test_request(&saved())).await);
    assert_eq!(error["extensions"]["fields"][0]["field"], "host");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_the_input_names_one_subject(pool: PgPool) {
    let schema = schema(&pool, &loopback_live());
    let cases = [
        (json!({}), "deviceId", "VALIDATION"),
        (
            json!({ "deviceId": PVE_01, "host": "10.0.0.1" }),
            "deviceId",
            "VALIDATION",
        ),
        (json!({ "deviceId": "not-an-id" }), "deviceId", "VALIDATION"),
        (
            json!({ "deviceId": PVE_01, "ssh": { "enabled": true, "port": 22, "username": USER,
            "auth": "PASSWORD" } }),
            "deviceId",
            "VALIDATION",
        ),
        (json!({ "host": "http://x/" }), "host", "VALIDATION"),
        (
            json!({ "host": "10.0.0.1", "ssh": { "enabled": true, "port": 0, "username": USER,
            "auth": "PASSWORD" } }),
            "ssh.port",
            "VALIDATION",
        ),
    ];
    for (input, field, code) in cases {
        let error = error_json(&schema.execute(test_request(&input)).await);
        assert_eq!(error["extensions"]["code"], code, "{input}");
        assert_eq!(error["extensions"]["fields"][0]["field"], field, "{input}");
    }
    let unknown = json!({ "deviceId": "0190a000-0000-7000-8002-0000000000ff" });
    let error = error_json(&schema.execute(test_request(&unknown)).await);
    assert_eq!(error["extensions"]["code"], "NOT_FOUND");
}
