//! `anima device create|update|delete|wake|test|metrics|uptime` against the scripted hub
//! (CMD-020, CMD-024, CMD-021).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-device-change-tests";
const SITE: &str = "0190a000-0000-7000-8001-000000000001";
const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

/// A saved device as `create.graphql` and `update.graphql` select it.
fn saved(name: &str) -> Value {
    json!({
        "id": DEVICE, "name": name, "host": "10.0.0.9", "kind": "server", "os": "LINUX",
        "tags": ["infra", "lab"], "status": "UNKNOWN", "site": { "id": SITE, "name": "home" },
    })
}

fn sent(hub: &FakeHub, operation: &str) -> Value {
    hub.seen()
        .into_iter()
        .find(|seen| seen.operation == operation)
        .unwrap_or_else(|| panic!("no {operation} request"))
        .variables
}

#[tokio::test]
async fn cmd_020_device_create_sends_the_flags_as_the_k1_input_and_prints_the_device() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DeviceCreate",
        data(json!({ "createDevice": saved("nas") })),
    );
    let run = signed_in()
        .run(&[
            "device",
            "create",
            "nas",
            "--site",
            SITE,
            "--host",
            "10.0.0.9",
            "--tag",
            "infra",
            "--tag",
            "lab",
            "--os",
            "linux",
            "--ssh-username",
            "operator",
            "--ssh-port",
            "2222",
            "--transfer-upload",
            "false",
            "--no-default-check",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        sent(&hub, "DeviceCreate")["input"],
        json!({
            "siteId": SITE, "name": "nas", "host": "10.0.0.9", "tags": ["infra", "lab"],
            "os": "LINUX", "addDefaultCheck": false,
            "ssh": { "enabled": true, "port": 2222, "username": "operator", "auth": "PASSWORD" },
            "transfer": { "upload": false },
        })
    );
    let name_line = run.stdout.lines().find(|line| line.starts_with("Name"));
    assert!(
        name_line.is_some_and(|line| line.ends_with("nas")),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains(DEVICE), "{}", run.stdout);

    let run = signed_in()
        .run(&[
            "device",
            "create",
            "nas",
            "--site",
            SITE,
            "--host",
            "10.0.0.9",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.json()["id"], DEVICE);
}

#[tokio::test]
async fn cmd_020_a_settings_group_without_its_user_name_is_refused_before_any_request() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&[
            "device",
            "create",
            "nas",
            "--site",
            SITE,
            "--host",
            "h",
            "--rdp-port",
            "3390",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--rdp-username"), "{}", run.stderr);
    assert!(hub.seen().is_empty());
}

#[tokio::test]
async fn cmd_021_a_forbidden_create_is_exit_5() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DeviceCreate",
        refusal("FORBIDDEN", "A read key cannot change devices."),
    );
    let run = signed_in()
        .run(&[
            "device",
            "create",
            "nas",
            "--site",
            SITE,
            "--host",
            "h",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    assert!(run.stderr.contains("A read key cannot change devices."));
}

#[tokio::test]
async fn cmd_020_device_update_sends_only_the_fields_that_have_a_flag() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DeviceUpdate",
        data(json!({ "updateDevice": saved("nas") })),
    );
    let run = signed_in()
        .run(&[
            "device",
            "update",
            DEVICE,
            "--maintenance",
            "true",
            "--clear-tags",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let variables = sent(&hub, "DeviceUpdate");
    assert_eq!(variables["id"], DEVICE);
    assert_eq!(
        variables["input"],
        json!({ "maintenance": true, "tags": [] })
    );

    let run = signed_in()
        .run(&["device", "update", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("Nothing to change"), "{}", run.stderr);
}

#[tokio::test]
async fn cmd_024_device_delete_needs_yes_and_then_prints_the_deleted_id() {
    let hub = FakeHub::start().await;
    hub.answer("DeviceDelete", data(json!({ "deleteDevice": DEVICE })));
    let run = signed_in()
        .run(&["device", "delete", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--yes"), "{}", run.stderr);
    assert!(
        hub.seen().is_empty(),
        "nothing is sent before the confirmation"
    );

    let run = signed_in()
        .run(&["device", "delete", DEVICE, "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains(DEVICE), "{}", run.stdout);
    assert_eq!(sent(&hub, "DeviceDelete")["id"], DEVICE);
}

#[tokio::test]
async fn cmd_020_device_wake_prints_the_relay_and_a_refusal_keeps_its_code() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DeviceWake",
        data(json!({ "wakeDevice": { "sentAt": "2026-11-13T09:00:00+00:00", "relayName": "pi-home" } })),
    );
    let run = signed_in()
        .run(&["device", "wake", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("pi-home"), "{}", run.stdout);
    assert_eq!(sent(&hub, "DeviceWake")["deviceId"], DEVICE);

    let refusing = FakeHub::start().await;
    refusing.answer(
        "DeviceWake",
        refusal("VALIDATION", "The site has no wake relay."),
    );
    let run = signed_in()
        .run(&["device", "wake", DEVICE, "--hub", &refusing.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("no wake relay"));
}

#[tokio::test]
async fn cmd_020_device_test_shows_the_host_key_and_the_command_that_trusts_it() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DeviceTest",
        data(json!({ "testConnection": {
            "reachable": true, "authenticated": false,
            "hostKey": { "algorithm": "ssh-ed25519", "fingerprintSha256": "SHA256:newkey",
                         "firstSeenAt": "2026-11-13T09:00:00+00:00" },
            "hostKeyChanged": false, "message": "The host key is not accepted yet.",
            "serverCertificate": null, "certificateChanged": false, "certificateMatched": false,
        }})),
    );
    let run = signed_in()
        .run(&[
            "device",
            "test",
            DEVICE,
            "--protocol",
            "ssh",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("ssh-ed25519 SHA256:newkey"),
        "{}",
        run.stdout
    );
    assert!(
        run.stderr.contains(&format!(
            "anima device trust accept-host-key {DEVICE} SHA256:newkey"
        )),
        "{}",
        run.stderr
    );
    assert_eq!(
        sent(&hub, "DeviceTest")["input"],
        json!({ "deviceId": DEVICE, "protocol": "SSH" })
    );
}

#[tokio::test]
async fn cmd_020_device_metrics_and_uptime_read_a_range_and_a_window() {
    let hub = FakeHub::start().await;
    let point = |value: f64| json!({ "at": "2026-11-13T09:00:00+00:00", "value": value, "min": null, "max": null });
    hub.answer(
        "DeviceMetrics",
        data(json!({ "device": { "id": DEVICE, "name": "nas", "metrics": [
            { "kind": "CPU_PERCENT", "label": "", "gpuIndex": null, "points": [point(10.0), point(30.0)] },
        ]}})),
    )
    .answer(
        "DeviceUptime",
        data(json!({ "device": { "id": DEVICE, "name": "nas", "uptime": 0.9991 } })),
    );
    let run = signed_in()
        .run(&[
            "device",
            "metrics",
            DEVICE,
            "--range",
            "day",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run
        .stdout
        .lines()
        .find(|line| line.starts_with("CPU_PERCENT"))
        .expect("a row");
    assert!(row.contains("30.00") && row.contains("10.00"), "{row}");
    assert_eq!(sent(&hub, "DeviceMetrics")["range"], "DAY");

    let run = signed_in()
        .run(&[
            "device",
            "uptime",
            DEVICE,
            "--window",
            "week",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("99.91 %"), "{}", run.stdout);
    assert_eq!(sent(&hub, "DeviceUptime")["window"], "WEEK");
}
