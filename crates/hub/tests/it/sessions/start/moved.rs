//! `startSession` after the device's host changed (SEC-011): a stored credential entered for
//! another host refuses the start with `CONFLICT` and one Warning event, and nothing starts;
//! entering the credential again, or changing the host back, lets it start; an unchanged host
//! is not refused, whatever its letter case.

use anima_hub::{
    credentials::{PasswordKind, Secret, SshSecret, store_password, store_ssh},
    ids::DeviceId,
    sessions::Live,
};
use serde_json::Value;
use sqlx::PgPool;

use crate::{
    hub::host_of,
    sessions::{
        PVE_01,
        harness::{TestSchema, error_json, schema_at, vault},
        now,
        starting::{display, protocol_start_request, ready, session_count},
    },
};

/// PVE-01's host in the fixture.
const PVE_01_HOST: &str = "pve-01.home.lan";
/// The host an attacker points PVE-01 at.
const CHOSEN: &str = "collector.example.net";

fn pve_01() -> DeviceId {
    PVE_01.parse().unwrap()
}

/// PVE-01 ready for SSH, with RDP (standard security, so no certificate is needed) and VNC on.
async fn desktop_ready(pool: &PgPool) {
    ready(pool, &[PVE_01]).await;
    for change in [
        "update devices.rdp_settings set enabled = true, security = 'rdp' where device_id = $1",
        "update devices.vnc_settings set enabled = true where device_id = $1",
    ] {
        sqlx::query(change)
            .bind(pve_01())
            .execute(pool)
            .await
            .unwrap();
    }
}

/// Stores every PVE-01 credential, bound to its host now, as the set mutations do.
async fn enter_credentials(pool: &PgPool) {
    let host = host_of(pool, pve_01()).await;
    let mut conn = pool.acquire().await.unwrap();
    let ssh = SshSecret::Password(Secret::from("ssh-SECRET".to_owned()));
    store_ssh(&mut conn, &vault(), (pve_01(), &host), ssh)
        .await
        .unwrap();
    for kind in [PasswordKind::Rdp, PasswordKind::Vnc] {
        let password = (kind, Secret::from("desktop-SECRET".to_owned()));
        store_password(&mut conn, &vault(), (pve_01(), &host), password)
            .await
            .unwrap();
    }
}

async fn move_to(pool: &PgPool, host: &str) {
    sqlx::query("update devices.devices set host = $2 where id = $1")
        .bind(pve_01())
        .bind(host)
        .execute(pool)
        .await
        .unwrap();
}

/// The `(severity, source, message)` of every event.
async fn events(pool: &PgPool) -> Vec<(String, String, String)> {
    sqlx::query_as("select severity, source, message from events.event_log")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn start(schema: &TestSchema, protocol: &str) -> async_graphql::Response {
    schema
        .execute(protocol_start_request(PVE_01, protocol, &display()))
        .await
}

async fn starts(schema: &TestSchema, protocol: &str) {
    let response = start(schema, protocol).await;
    assert!(
        response.errors.is_empty(),
        "{protocol}: {:?}",
        response.errors
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_011_a_moved_credential_refuses_the_start_with_one_warning(pool: PgPool) {
    desktop_ready(&pool).await;
    enter_credentials(&pool).await;
    move_to(&pool, CHOSEN).await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    for (protocol, credential) in [
        ("SSH", "SSH credential"),
        ("RDP", "RDP password"),
        ("VNC", "VNC password"),
    ] {
        let error = error_json(&start(&schema, protocol).await);
        assert_eq!(error["extensions"]["code"], "CONFLICT", "{protocol}");
        let message = error["message"].as_str().unwrap();
        assert!(message.contains("address changed"), "{message}");
        assert!(message.contains(credential), "{message}");
        assert!(
            !message.contains(CHOSEN) && !message.contains(PVE_01_HOST),
            "{message}"
        );
        assert!(!message.contains("SECRET"), "{message}");
        let written = events(&pool).await;
        let last = written.last().unwrap();
        assert_eq!((last.0.as_str(), last.1.as_str()), ("warning", "session"));
        assert_eq!(last.2, message);
    }
    assert_eq!(events(&pool).await.len(), 3, "one event per refused start");
    assert_eq!(session_count(&pool).await, 0, "nothing started");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_011_rdp_is_refused_in_every_security_mode(pool: PgPool) {
    desktop_ready(&pool).await;
    enter_credentials(&pool).await;
    move_to(&pool, CHOSEN).await;
    let schema = schema_at(&pool, now(), &Live::default());
    for settings in [
        "security = 'rdp', ignore_certificate = false",
        "security = 'nla', ignore_certificate = true",
        "security = 'any', ignore_certificate = true",
    ] {
        sqlx::query(&format!(
            "update devices.rdp_settings set {settings} where device_id = $1"
        ))
        .bind(pve_01())
        .execute(&pool)
        .await
        .unwrap();
        let error = error_json(&start(&schema, "RDP").await);
        assert_eq!(error["extensions"]["code"], "CONFLICT", "{settings}");
    }
    assert_eq!(session_count(&pool).await, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_011_entering_the_credential_again_lets_the_start_go_on(pool: PgPool) {
    desktop_ready(&pool).await;
    enter_credentials(&pool).await;
    move_to(&pool, CHOSEN).await;
    let schema = schema_at(&pool, now(), &Live::default());
    assert_eq!(
        error_json(&start(&schema, "VNC").await)["extensions"]["code"],
        "CONFLICT"
    );
    enter_credentials(&pool).await;
    for protocol in ["SSH", "RDP", "VNC"] {
        starts(&schema, protocol).await;
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_011_changing_the_host_back_needs_no_new_entry(pool: PgPool) {
    desktop_ready(&pool).await;
    enter_credentials(&pool).await;
    move_to(&pool, CHOSEN).await;
    let schema = schema_at(&pool, now(), &Live::default());
    let refused: Value = error_json(&start(&schema, "RDP").await);
    assert_eq!(refused["extensions"]["code"], "CONFLICT");
    move_to(&pool, PVE_01_HOST).await;
    for protocol in ["SSH", "RDP", "VNC"] {
        starts(&schema, protocol).await;
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_011_an_unchanged_host_is_not_refused_whatever_its_letter_case(pool: PgPool) {
    desktop_ready(&pool).await;
    enter_credentials(&pool).await;
    let schema = schema_at(&pool, now(), &Live::default());
    starts(&schema, "VNC").await;
    move_to(&pool, "PVE-01.Home.LAN").await;
    for protocol in ["SSH", "RDP", "VNC"] {
        starts(&schema, protocol).await;
    }
    assert!(events(&pool).await.is_empty(), "no refusal was written");
}
