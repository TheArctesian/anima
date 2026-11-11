//! `startSession` and a device that records its sessions (RA-040, K4 §1 step 2, D-14): the
//! start is refused with UNAVAILABLE when the hub cannot record, and the refusal names no path;
//! a recorded start says so in its ticket and its audit entry; a device that does not record is
//! never checked. Low free space is refused by the same rule (`sessions::tests::start`).

use std::{fs, os::unix::fs::PermissionsExt as _, path::PathBuf};

use anima_hub::sessions::Live;
use async_graphql::{Request, Response, Variables};
use serde_json::{Value, json};
use sqlx::PgPool;
use tempfile::TempDir;

use crate::sessions::{
    PVE_01,
    harness::{TestSchema, error_json, schema_recording_to, signed_in_until},
    now,
    starting::{display, ready, redeem, session_audits, session_count},
};

const START: &str = "mutation($input: StartSessionInput!) { startSession(input: $input) { \
                     sessionId token recording } }";

const CANNOT_RECORD: &str = "Recording is on for this device, but the hub cannot record now.";

fn start_request(device: &str) -> Request {
    let input = json!({ "deviceId": device, "protocol": "SSH", "display": display() });
    signed_in_until(START, 24).variables(Variables::from_json(json!({ "input": input })))
}

async fn start(schema: &TestSchema) -> Response {
    schema.execute(start_request(PVE_01)).await
}

/// The ticket of a start that must succeed.
async fn ticket(schema: &TestSchema) -> Value {
    let response = start(schema).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["startSession"].clone()
}

async fn set_recording(pool: &PgPool, on: bool) {
    sqlx::query("update devices.devices set record_sessions = $2 where id = $1::text::uuid")
        .bind(PVE_01)
        .bind(on)
        .execute(pool)
        .await
        .unwrap();
}

/// A folder in the build's own temporary space, which has room for the 1 GiB check.
fn root() -> TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap()
}

fn schema(pool: &PgPool, live: &Live, root: Option<PathBuf>) -> TestSchema {
    schema_recording_to(pool, now(), live, root)
}

/// Asserts the start was refused as K4 §1 step 2 says, its error names no part of the
/// `root` folder, and it wrote no row and no audit entry.
async fn assert_refused(pool: &PgPool, response: &Response, root: Option<&PathBuf>) {
    let error = error_json(response);
    assert_eq!(error["extensions"]["code"], "UNAVAILABLE", "{error}");
    assert_eq!(error["message"], CANNOT_RECORD, "{error}");
    let text = error.to_string();
    for part in root.iter().flat_map(|root| root.iter()).skip(1) {
        let part = part.to_str().unwrap();
        assert!(!text.contains(part), "LOG-007: {part:?} in {text}");
    }
    assert_eq!(session_count(pool).await, 0, "the refusal wrote no row");
    assert_eq!(session_audits(pool).await.len(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn d_14_a_recorded_device_is_refused_when_no_recording_root_is_set(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_recording(&pool, true).await;
    let response = start(&schema(&pool, &Live::default(), None)).await;
    assert_refused(&pool, &response, None).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn d_14_a_recorded_device_is_refused_when_the_root_is_read_only(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_recording(&pool, true).await;
    let folder = root();
    let read_only = folder.path().join("recordings");
    fs::create_dir(&read_only).unwrap();
    fs::set_permissions(&read_only, fs::Permissions::from_mode(0o555)).unwrap();

    let response = start(&schema(&pool, &Live::default(), Some(read_only.clone()))).await;

    fs::set_permissions(&read_only, fs::Permissions::from_mode(0o755)).unwrap();
    assert_refused(&pool, &response, Some(&read_only)).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn d_14_a_recorded_device_is_refused_when_the_root_is_missing(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_recording(&pool, true).await;
    let folder = root();
    let missing = folder.path().join("no-such-folder");
    let response = start(&schema(&pool, &Live::default(), Some(missing.clone()))).await;
    assert_refused(&pool, &response, Some(&missing)).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_040_a_recorded_start_says_so_in_its_ticket_and_audit_entry(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_recording(&pool, true).await;
    let folder = root();
    let live = Live::default();

    let ticket = ticket(&schema(&pool, &live, Some(folder.path().to_owned()))).await;

    assert_eq!(ticket["recording"], true);
    let audits = session_audits(&pool).await;
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].action, "start");
    assert_eq!(
        audits[0].change,
        json!({ "deviceId": PVE_01, "deviceName": "pve-01", "protocol": "ssh", "recorded": true })
    );
    let redeemed = redeem(&live, &ticket, now()).expect("the ticket opens the tunnel");
    assert!(redeemed.ticket.recording, "the tunnel learns it records");
    assert_eq!(
        fs::read_dir(folder.path()).unwrap().count(),
        0,
        "the start writes no file (G-4)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_040_a_device_that_does_not_record_starts_unrecorded_without_a_check(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_recording(&pool, false).await;
    let live = Live::default();

    // No root: a check would refuse the start.
    let ticket = ticket(&schema(&pool, &live, None)).await;

    assert_eq!(ticket["recording"], false);
    let audits = session_audits(&pool).await;
    assert_eq!(
        audits[0].change,
        json!({ "deviceId": PVE_01, "deviceName": "pve-01", "protocol": "ssh" }),
        "no `recorded` key"
    );
    let redeemed = redeem(&live, &ticket, now()).expect("the ticket opens the tunnel");
    assert!(!redeemed.ticket.recording);
}
