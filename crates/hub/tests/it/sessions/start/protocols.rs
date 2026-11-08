//! `startSession` for RDP and VNC (K4 §1): each needs its protocol on; RDP also needs an
//! accepted server certificate unless its security is `RDP` or the certificate is ignored;
//! neither needs an SSH host key. A refusal writes nothing.

use anima_hub::sessions::Live;
use serde_json::Value;
use sqlx::PgPool;

use crate::sessions::{
    OPERATOR_ID, PVE_01, add_operator,
    harness::{TestSchema, error_json, schema_at},
    now,
    starting::{display, protocol_start_request, ready, session_audits, session_count},
};

/// Sets `columns` (an SQL `set` list) on PVE-01's settings `table`.
async fn set(pool: &PgPool, table: &str, columns: &str) {
    sqlx::query(&format!(
        "update devices.{table} set {columns} where device_id = $1::text::uuid"
    ))
    .bind(PVE_01)
    .execute(pool)
    .await
    .unwrap();
}

/// Accepts a server certificate for PVE-01.
async fn accept_certificate(pool: &PgPool) {
    sqlx::query(
        "insert into trust.server_certificates (device_id, fingerprint_sha256, \
                first_seen_at, accepted_by) \
         values ($1::text::uuid, $2, $3, $4::text::uuid)",
    )
    .bind(PVE_01)
    .bind(vec!["ab"; 32].join(":"))
    .bind(now())
    .bind(OPERATOR_ID)
    .execute(pool)
    .await
    .unwrap();
}

/// The refusal of a `protocol` start on PVE-01: its code and message.
async fn refusal(schema: &TestSchema, protocol: &str) -> (Value, Value) {
    let response = schema
        .execute(protocol_start_request(PVE_01, protocol, &display()))
        .await;
    let error = error_json(&response);
    (
        error["extensions"]["code"].clone(),
        error["message"].clone(),
    )
}

/// The ticket of a `protocol` start on PVE-01, as K1 shapes it.
async fn ticket(schema: &TestSchema, protocol: &str) -> Value {
    let response = schema
        .execute(protocol_start_request(PVE_01, protocol, &display()))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["startSession"].clone()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn k4_1_start_session_refuses_rdp_and_vnc_while_they_are_off(pool: PgPool) {
    // SSH is on and a host key is accepted, so only the protocol can refuse the start.
    ready(&pool, &[PVE_01]).await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    for (protocol, message) in [
        (
            "RDP",
            "RDP is turned off for this device. Turn it on in the device's RDP settings.",
        ),
        (
            "VNC",
            "VNC is turned off for this device. Turn it on in the device's VNC settings.",
        ),
    ] {
        let (code, text) = refusal(&schema, protocol).await;
        assert_eq!(
            (code.as_str(), text.as_str()),
            (Some("CONFLICT"), Some(message))
        );
    }
    assert_eq!(session_count(&pool).await, 0);
    assert_eq!(session_audits(&pool).await.len(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_rdp_without_an_accepted_certificate_is_a_conflict(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set(&pool, "rdp_settings", "enabled = true").await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    for security in ["nla", "nla_ext", "tls", "any", "vmconnect"] {
        set(&pool, "rdp_settings", &format!("security = '{security}'")).await;
        let (code, text) = refusal(&schema, "RDP").await;
        assert_eq!(code, "CONFLICT", "{security}");
        assert_eq!(
            text,
            "No server certificate is accepted for this device. Test the connection and accept \
             its certificate."
        );
    }
    assert_eq!(session_count(&pool).await, 0);
    assert_eq!(session_audits(&pool).await.len(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn k4_1_rdp_starts_with_a_certificate_or_when_none_is_checked(pool: PgPool) {
    // No SSH host key: RDP does not need one.
    add_operator(&pool).await;
    set(&pool, "rdp_settings", "enabled = true").await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    set(&pool, "rdp_settings", "security = 'rdp'").await;
    let started = ticket(&schema, "RDP").await;
    assert_eq!(started["tunnelPath"], "/tunnel");
    assert_eq!(started["wakeSent"], false);
    set(
        &pool,
        "rdp_settings",
        "security = 'nla', ignore_certificate = true",
    )
    .await;
    ticket(&schema, "RDP").await;
    set(&pool, "rdp_settings", "ignore_certificate = false").await;
    accept_certificate(&pool).await;
    ticket(&schema, "RDP").await;
    assert_eq!(session_count(&pool).await, 3);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn k4_1_vnc_starts_when_on_without_a_host_key_or_certificate(pool: PgPool) {
    add_operator(&pool).await;
    set(&pool, "vnc_settings", "enabled = true").await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let started = ticket(&schema, "VNC").await;
    assert_eq!(started["tunnelPath"], "/tunnel");
    assert_eq!(started["wakeSent"], false);
    assert_eq!(session_count(&pool).await, 1);
}
