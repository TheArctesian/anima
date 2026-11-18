//! `anima daemon` against the scripted hub (CMD-020, CMD-021, CMD-024, R-122).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-daemon-tests";
const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
const CODE: &str = "enrol-code-0f3a9c71d2";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

fn variables(hub: &FakeHub, operation: &str) -> Value {
    hub.seen()
        .into_iter()
        .find(|seen| seen.operation == operation)
        .unwrap_or_else(|| panic!("no {operation} request"))
        .variables
}

#[tokio::test]
async fn cmd_020_daemon_status_says_whether_the_daemon_is_reporting() {
    let hub = FakeHub::start().await;
    let device = |stale: bool| {
        data(
            json!({ "device": { "id": DEVICE, "name": "nas", "metricsSource": "DAEMON",
            "metricsStale": stale, "readings": { "at": "2026-11-13T09:00:00+00:00" } } }),
        )
    };
    hub.answer("DaemonStatus", device(false))
        .answer("DaemonStatus", device(true));
    let run = signed_in()
        .run(&["daemon", "status", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("reporting") && run.stdout.contains("2026-11-13T09:00:00"),
        "{}",
        run.stdout
    );

    let run = signed_in()
        .run(&["daemon", "status", DEVICE, "--hub", &hub.origin])
        .await;
    assert!(run.stdout.contains("stale"), "{}", run.stdout);

    let run = signed_in()
        .run(&["daemon", "status", DEVICE, "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["metricsSource"], "DAEMON");
}

#[tokio::test]
async fn r_122_daemon_status_gives_the_version_the_daemon_reported() {
    let hub = FakeHub::start().await;
    let device = |version: Value| {
        data(
            json!({ "device": { "id": DEVICE, "name": "nas", "metricsSource": "DAEMON",
            "metricsStale": false, "daemonVersion": version, "readings": null } }),
        )
    };
    hub.answer("DaemonStatus", device(json!("0.3.1")))
        .answer("DaemonStatus", device(Value::Null))
        .answer("DaemonStatus", device(json!("0.3.1")));
    let line = |stdout: &str| {
        stdout
            .lines()
            .find(|line| line.starts_with("Daemon version"))
            .map(str::to_owned)
    };
    let run = signed_in()
        .run(&["daemon", "status", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        line(&run.stdout).is_some_and(|line| line.ends_with("0.3.1")),
        "{}",
        run.stdout
    );

    let run = signed_in()
        .run(&["daemon", "status", DEVICE, "--hub", &hub.origin])
        .await;
    assert!(
        line(&run.stdout).is_some_and(|line| line.ends_with("not reported")),
        "a daemon older than R-122: {}",
        run.stdout
    );

    let run = signed_in()
        .run(&["daemon", "status", DEVICE, "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["daemonVersion"], "0.3.1");
}

#[tokio::test]
async fn met_012_daemon_enrol_prints_the_one_time_code_and_its_expiry() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DaemonEnrol",
        data(json!({ "enrolDaemon": { "code": CODE, "expiresAt": "2026-11-13T09:15:00+00:00" } })),
    );
    let run = signed_in()
        .run(&["daemon", "enrol", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout, format!("{CODE}\n"), "the code alone on stdout");
    assert!(run.stderr.contains("2026-11-13T09:15:00"), "{}", run.stderr);
    assert!(run.stderr.contains("shown one time"), "{}", run.stderr);
    assert!(
        !run.stderr.contains(CODE),
        "the code is printed once, on standard output"
    );
    assert_eq!(variables(&hub, "DaemonEnrol")["deviceId"], DEVICE);

    let run = signed_in()
        .run(&["daemon", "enrol", DEVICE, "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["code"], CODE);
    assert_eq!(run.json()["expiresAt"], "2026-11-13T09:15:00+00:00");
}

#[tokio::test]
async fn cmd_021_an_api_key_cannot_enrol_a_daemon() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DaemonEnrol",
        refusal("FORBIDDEN", "Only a login token may do this."),
    );
    let run = signed_in()
        .run(&["daemon", "enrol", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    run.assert_never_shows(CODE);
}

#[tokio::test]
async fn cmd_024_daemon_revoke_needs_yes_and_prints_the_stale_source() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DaemonRevoke",
        data(json!({ "revokeDaemonToken": { "id": DEVICE, "name": "nas",
            "metricsSource": "DAEMON", "metricsStale": true } })),
    );
    let run = signed_in()
        .run(&["daemon", "revoke", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(hub.seen().is_empty());

    let run = signed_in()
        .run(&["daemon", "revoke", DEVICE, "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("DAEMON") && run.stdout.contains("yes"),
        "{}",
        run.stdout
    );
    assert_eq!(variables(&hub, "DaemonRevoke")["deviceId"], DEVICE);
}
