//! Deleting a device removes everything that belongs to it and keeps
//! what the operator must still be able to read (DEV-015, RA-008, LOG-005, R-004).

mod canary;

use anima_hub::{
    checks::{self, Beat, CheckState, Recorded},
    ids::CheckId,
    status::Status,
};
use chrono::{DateTime, Days, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start};

const HOME_SITE: &str = "0190a000-0000-7000-8001-000000000001";
const ED25519: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIPTJnBUBJyxtaF5R3VPI+WZbUsZP0DeocOVAgFsBtx/m";
const ED25519_FINGERPRINT: &str = "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk";

const CREATE_DEVICE: &str =
    "mutation($input: CreateDeviceInput!) { createDevice(input: $input) { id } }";
const SET_CREDENTIAL: &str = "mutation($id: ID!, $c: SshCredentialInput!) { \
    setSshCredential(deviceId: $id, credential: $c) { credentialStored } }";
const CREATE_CHECK: &str =
    "mutation($input: CreateCheckInput!) { createCheck(input: $input) { id } }";
const DELETE_DEVICE: &str = "mutation($id: ID!) { deleteDevice(id: $id) }";

/// Everything of the device that must go with it, each with the count query for its id.
/// The secret goes by `clear_ssh` and by the secrets foreign key's cascade; each alone is
/// covered by `credentials::schema::dev_015_deleting_the_device_deletes_its_secrets` and
/// `devices::credential::dev_015_deleting_a_device_removes_its_secret`.
const CHAIN: [(&str, &str); 6] = [
    (
        "device",
        "select count(*) from devices.devices where id = $1::text::uuid",
    ),
    (
        "tags",
        "select count(*) from devices.device_tags where device_id = $1::text::uuid",
    ),
    (
        "ssh settings",
        "select count(*) from devices.ssh_settings where device_id = $1::text::uuid",
    ),
    (
        "credential",
        "select count(*) from credentials.secrets where device_id = $1::text::uuid",
    ),
    (
        "checks",
        "select count(*) from checks.checks where device_id = $1::text::uuid",
    ),
    (
        "host key",
        "select count(*) from trust.host_keys where device_id = $1::text::uuid",
    ),
];

/// Each rollup table that holds a row per check, counted by the check's id (DB-008).
const UPTIME: [(&str, &str); 3] = [
    (
        "uptime_minute",
        "select count(*) from checks.uptime_minute where check_id = $1::text::uuid",
    ),
    (
        "uptime_hour",
        "select count(*) from checks.uptime_hour where check_id = $1::text::uuid",
    ),
    (
        "uptime_day",
        "select count(*) from checks.uptime_day where check_id = $1::text::uuid",
    ),
];

async fn count(pool: &PgPool, sql: &str, id: &str) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn up_beat(at: DateTime<Utc>) -> Beat {
    Beat {
        at,
        status: Status::Up,
        response_time_ms: Some(12),
        message: None,
        state: CheckState {
            status: Status::Up,
            failures: 0,
            next_due_at: Some(at + TimeDelta::seconds(60)),
        },
        resolved_ip: None,
        cert_not_after: None,
        domain_not_after: None,
    }
}

fn id_of(response: &Value, field: &str) -> String {
    response["data"][field]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{field} failed: {response}"))
        .to_owned()
}

/// Gives the device a stored accepted host key and one ended session.
async fn add_host_key_and_session(pool: &PgPool, device: &str) {
    sqlx::query(
        "insert into trust.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
             first_seen_at, accepted_at, accepted_by) \
         select $1::text::uuid, 'ssh-ed25519', $2, $3, now(), now(), id from access.operators",
    )
    .bind(device)
    .bind(ED25519_FINGERPRINT)
    .bind(ED25519)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into sessions.sessions (id, device_id, device_name, protocol, operator_id, \
             operator_name, client_addr, started_at, ended_at, end_reason) \
         select '0190a000-0000-7000-8010-000000000001', $1::text::uuid, 'pve-99', 'ssh', id, \
             username, '10.10.0.77', now(), now(), 'operator' from access.operators",
    )
    .bind(device)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn dev_015_deleting_a_device_removes_its_whole_chain_and_keeps_its_history(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let token = Some(token.as_str());
    let input = json!({ "siteId": HOME_SITE, "name": "pve-99", "host": "pve-99.home.lan",
        "tags": ["edge", "lab"], "addDefaultCheck": false,
        "ssh": { "enabled": true, "username": "operator", "auth": "PASSWORD" } });
    let created = hub
        .post(token, CREATE_DEVICE, json!({ "input": input }))
        .await;
    let device = id_of(&created, "createDevice");
    let secret = json!({ "id": device, "c": { "password": "a long ssh password" } });
    hub.post(token, SET_CREDENTIAL, secret).await;
    let web = json!({ "deviceId": device, "name": "Web", "config": { "tcp": { "port": 22 } } });
    let made = hub.post(token, CREATE_CHECK, json!({ "input": web })).await;
    let check_id = id_of(&made, "createCheck");
    let check: CheckId = check_id.parse().unwrap();

    let db_now: DateTime<Utc> = sqlx::query_scalar("select now()")
        .fetch_one(&pool)
        .await
        .unwrap();
    checks::drop_expired(&pool, db_now, Days::new(30))
        .await
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    for second in 0..3 {
        let beat = up_beat(db_now + TimeDelta::seconds(second));
        let recorded = checks::record_beat(&mut conn, check, &beat).await.unwrap();
        assert_eq!(recorded, Recorded::Written);
    }
    drop(conn);
    add_host_key_and_session(&pool, &device).await;
    for (what, sql) in CHAIN {
        assert!(
            count(&pool, sql, &device).await >= 1,
            "before the delete: {what}"
        );
    }
    for (what, sql) in UPTIME {
        assert!(
            count(&pool, sql, &check_id).await >= 1,
            "before the delete: {what}"
        );
    }
    let beats = "select count(*) from checks.heartbeats where check_id = $1::text::uuid";
    assert_eq!(count(&pool, beats, &check_id).await, 3);

    let deleted = hub
        .post(token, DELETE_DEVICE, json!({ "id": device }))
        .await;
    assert_eq!(
        deleted["data"]["deleteDevice"],
        device.as_str(),
        "{deleted}"
    );

    for (what, sql) in CHAIN {
        assert_eq!(
            count(&pool, sql, &device).await,
            0,
            "after the delete: {what}"
        );
    }
    for (what, sql) in UPTIME {
        assert_eq!(
            count(&pool, sql, &check_id).await,
            0,
            "after the delete: {what}"
        );
    }
    let history = "select count(*) from sessions.sessions where device_id = $1::text::uuid";
    assert_eq!(
        count(&pool, history, &device).await,
        1,
        "session history stays (RA-008)"
    );
    let audits = "select count(*) from events.audit_log where target_id = $1 \
                  and action like '%delete%'";
    let events = "select count(*) from events.event_log where device_id = $1::text::uuid \
                  and message like '%deleted%'";
    assert_eq!(
        count(&pool, audits, &device).await,
        1,
        "one audit entry (LOG-005)"
    );
    assert_eq!(
        count(&pool, events, &device).await,
        1,
        "one event (LOG-005)"
    );
    assert_eq!(
        count(&pool, beats, &check_id).await,
        3,
        "beats wait for the sweep"
    );

    checks::drop_expired(&pool, db_now, Days::new(30))
        .await
        .unwrap();
    assert_eq!(
        count(&pool, beats, &check_id).await,
        0,
        "the sweep removes them (R-004)"
    );
}
