//! `startSession` for RDP and VNC before devices have their settings: each is off for every
//! device, so the hub refuses it and writes nothing (WP-22 D-3, K4 §1).

use anima_hub::sessions::Live;
use sqlx::PgPool;

use crate::sessions::{
    PVE_01,
    harness::{error_json, schema_at},
    now,
    starting::{display, protocol_start_request, ready, session_audits, session_count},
};

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
        ("RDP", "RDP is turned off for this device."),
        ("VNC", "VNC is turned off for this device."),
    ] {
        let response = schema
            .execute(protocol_start_request(PVE_01, protocol, &display()))
            .await;
        let error = error_json(&response);
        assert_eq!(error["extensions"]["code"], "CONFLICT", "{error}");
        assert_eq!(error["message"], message);
    }
    assert_eq!(session_count(&pool).await, 0);
    assert_eq!(session_audits(&pool).await.len(), 0);

    // The same device still starts over SSH, so the refusal is the protocol's alone.
    let response = schema
        .execute(protocol_start_request(PVE_01, "SSH", &display()))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(session_count(&pool).await, 1);
}
