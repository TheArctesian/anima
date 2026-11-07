//! The deny list on `testConnection` (R-030, R-032 E-7, security): a host the operator
//! types or a saved device's host that is, or resolves into, an `ANIMA_PROBE_DENY` network is
//! the `Validation` error on `host`, and no connection reaches it; a host outside the list is
//! still tested.

use std::time::Duration;

use anima_hub::{sessions::Live, targets::AddressPolicy};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{net::TcpListener, time::timeout};

use super::{
    fingerprint, loopback_live, schema,
    server::{SshServer, USER, ed25519},
    test_request, tested,
};
use crate::sessions::{PVE_01, harness::error_json};

/// The tests' loopback rule, with `cidr` denied.
fn denying(cidr: &str) -> Live {
    loopback_live().denying(AddressPolicy::new(vec![cidr.parse().unwrap()]))
}

fn unsaved(host: &str, port: u16) -> Value {
    json!({ "host": host, "ssh": { "enabled": true, "port": port, "username": USER,
        "auth": "PASSWORD" } })
}

/// Asserts the test was refused on `host` with a fixed text that names no address.
fn refused_on_host(error: &Value, subject: &str) {
    assert_eq!(
        error["extensions"]["code"], "VALIDATION",
        "{subject}: {error}"
    );
    let field = &error["extensions"]["fields"][0];
    assert_eq!(field["field"], "host", "{subject}");
    let message = field["message"].as_str().unwrap();
    assert!(message.contains("ANIMA_PROBE_DENY"), "{message}");
    assert!(
        !message.contains("127."),
        "no address in the message: {message}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn r_030_a_host_in_the_deny_list_is_refused_and_never_reached(pool: PgPool) {
    let device = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = device.local_addr().unwrap().port();
    let schema = schema(&pool, &denying("127.0.0.0/8"));
    for host in ["127.0.0.1", "localhost", "::ffff:127.0.0.1"] {
        let error = error_json(&schema.execute(test_request(&unsaved(host, port))).await);
        refused_on_host(&error, host);
    }
    sqlx::query("update devices.devices set host = '127.0.0.1' where id = $1::text::uuid")
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("update devices.ssh_settings set port = $2 where device_id = $1::text::uuid")
        .bind(PVE_01)
        .bind(i32::from(port))
        .execute(&pool)
        .await
        .unwrap();
    let saved = json!({ "deviceId": PVE_01 });
    refused_on_host(
        &error_json(&schema.execute(test_request(&saved)).await),
        "saved",
    );
    let reached = timeout(Duration::from_millis(50), device.accept()).await;
    assert!(reached.is_err(), "no connection reached the device");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn r_030_a_host_outside_the_deny_list_is_still_tested(pool: PgPool) {
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    let schema = schema(&pool, &denying("192.0.2.0/24"));

    let result = tested(&schema, &unsaved("127.0.0.1", server.port)).await;

    assert_eq!(result["reachable"], true);
    assert_eq!(
        result["hostKey"]["fingerprintSha256"],
        fingerprint(&host_key)
    );
}
