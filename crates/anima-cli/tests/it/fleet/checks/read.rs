//! `check list`, `check get` and `check heartbeats` (CMD-020, CMD-026).

use serde_json::{Value, json};

use super::{CHECK, DEVICE, operations, signed_in, variables};
use crate::kit::fake_hub::{FakeHub, data, refusal};

const PUSH_PATH: &str = "/push/secret-push-token-123";

fn list_answer() -> Value {
    data(json!({ "device": { "id": DEVICE, "name": "pve", "checks": [
        { "id": "0190a000-0000-7000-8003-000000000001", "name": "Ping", "type": "ICMP",
          "status": "UP", "paused": false, "intervalSeconds": 60,
          "config": { "__typename": "IcmpConfig", "ipVersion": "V4" } },
        { "id": CHECK, "name": "SSH port", "type": "TCP", "status": "DOWN", "paused": false,
          "intervalSeconds": 30, "config": { "__typename": "TcpConfig", "port": 22 } },
        { "id": "0190a000-0000-7000-8003-000000000003", "name": "Backup", "type": "PUSH",
          "status": "UNKNOWN", "paused": true, "intervalSeconds": 60,
          "config": { "__typename": "PushConfig" } },
    ]}}))
}

fn get_answer() -> Value {
    data(json!({ "check": {
        "id": CHECK, "name": "SSH port", "type": "TCP", "status": "DOWN", "paused": false,
        "invert": false, "intervalSeconds": 30, "retries": 3, "retryIntervalSeconds": 20,
        "degradedThresholdMs": 1000, "certificateDaysLeft": null, "domainDaysLeft": null,
        "device": { "id": DEVICE, "name": "pve" }, "parent": null,
        "config": { "__typename": "TcpConfig", "port": 22, "timeoutMs": 5000 },
        "notifications": { "resendEveryBeats": 5,
            "channels": [{ "id": "c1", "name": "ntfy", "kind": "NTFY" }] },
        "lastHeartbeat": { "at": "2026-11-12T10:00:00+00:00", "status": "DOWN",
            "responseTimeMs": null, "message": "connection refused" },
        "uptime": 0.5,
    }}))
}

#[tokio::test]
async fn cmd_020_check_list_prints_kind_and_key_settings() {
    let hub = FakeHub::start().await;
    hub.answer("CheckList", list_answer());
    let run = signed_in()
        .run(&["check", "list", "--device", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let header = run.stdout.lines().next().expect("a header");
    assert!(header.starts_with("NAME") && header.contains("KIND") && header.contains("SETTINGS"));
    let ssh = run
        .stdout
        .lines()
        .find(|line| line.starts_with("SSH port"))
        .expect("a row");
    assert!(
        ssh.contains("TCP") && ssh.contains("DOWN") && ssh.contains("port=22"),
        "{ssh}"
    );
    let backup = run
        .stdout
        .lines()
        .find(|line| line.starts_with("Backup"))
        .expect("a row");
    assert!(backup.contains("PAUSED"), "{backup}");
    assert_eq!(variables(&hub, "CheckList")["device"], DEVICE);

    let run = signed_in()
        .run(&[
            "check",
            "list",
            "--device",
            DEVICE,
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.json()["checks"][1]["config"]["port"], 22);
}

#[tokio::test]
async fn cmd_020_check_get_by_name_shows_the_settings_and_the_last_heartbeat() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CheckNames",
        data(json!({ "device": { "checks": [
            { "id": "0190a000-0000-7000-8003-000000000001", "name": "Ping" },
            { "id": CHECK, "name": "SSH port" },
        ]}})),
    )
    .answer("CheckGet", get_answer());
    let run = signed_in()
        .run(&[
            "check",
            "get",
            "SSH port",
            "--device",
            DEVICE,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    for expected in [
        "TCP",
        "Port",
        "22",
        "Timeout ms",
        "5000",
        "ntfy",
        "connection refused",
    ] {
        assert!(run.stdout.contains(expected), "{expected}: {}", run.stdout);
    }
    assert_eq!(variables(&hub, "CheckGet")["id"], CHECK);
}

#[tokio::test]
async fn cmd_020_check_get_is_the_only_command_that_shows_a_push_path() {
    let hub = FakeHub::start().await;
    let mut answer = get_answer();
    answer["data"]["check"]["type"] = json!("PUSH");
    answer["data"]["check"]["config"] =
        json!({ "__typename": "PushConfig", "pushPath": PUSH_PATH });
    hub.answer("CheckGet", answer)
        .answer("CheckList", list_answer());
    let run = signed_in()
        .run(&["check", "get", CHECK, "--hub", &hub.origin])
        .await;
    assert!(run.stdout.contains(PUSH_PATH), "{}", run.stdout);
    let run = signed_in()
        .run(&["check", "list", "--device", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    run.assert_never_shows(PUSH_PATH);
}

#[tokio::test]
async fn cmd_021_a_check_name_without_its_device_is_a_usage_error() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&["check", "get", "SSH port", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--device"), "{}", run.stderr);
    assert_eq!(operations(&hub), Vec::<String>::new());
}

#[tokio::test]
async fn cmd_021_a_refused_check_read_exits_5() {
    let hub = FakeHub::start().await;
    hub.answer("CheckGet", refusal("FORBIDDEN", "Not allowed."));
    let run = signed_in()
        .run(&["check", "get", CHECK, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
}

fn beats(cursor: &str, next: bool, at: &str) -> Value {
    data(json!({ "check": { "heartbeats": {
        "edges": [{ "cursor": cursor, "node": { "at": at, "status": "UP",
            "responseTimeMs": 12, "message": null } }],
        "pageInfo": { "hasNextPage": next, "endCursor": cursor },
    }}}))
}

#[tokio::test]
async fn cmd_026_heartbeats_page_with_limit_and_after_and_follow_with_all() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CheckHeartbeats",
        beats("h1.2", true, "2026-11-12T10:00:00+00:00"),
    );
    let run = signed_in()
        .run(&[
            "check",
            "heartbeats",
            CHECK,
            "--limit",
            "1",
            "--after",
            "h1.9",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("12 ms"), "{}", run.stdout);
    assert!(run.stderr.contains("--after h1.2"), "{}", run.stderr);
    let sent = variables(&hub, "CheckHeartbeats");
    assert_eq!(
        (sent["first"].clone(), sent["after"].clone()),
        (json!(1), json!("h1.9"))
    );

    let all = FakeHub::start().await;
    all.answer(
        "CheckHeartbeats",
        beats("h1.2", true, "2026-11-12T10:00:00+00:00"),
    )
    .answer(
        "CheckHeartbeats",
        beats("h1.1", false, "2026-11-12T09:59:00+00:00"),
    );
    let run = signed_in()
        .run(&[
            "check",
            "heartbeats",
            CHECK,
            "--all",
            "--json",
            "--hub",
            &all.origin,
        ])
        .await;
    assert_eq!(run.json()["edges"].as_array().expect("edges").len(), 2);
    assert_eq!(all.seen()[1].variables["after"], "h1.2");
}

#[tokio::test]
async fn cmd_026_the_next_page_hint_keeps_the_device_of_a_check_given_by_name() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CheckNames",
        data(json!({ "device": { "checks": [{ "id": CHECK, "name": "HTTP check" }] }})),
    );
    hub.answer(
        "CheckHeartbeats",
        beats("h1.2", true, "2026-11-12T10:00:00+00:00"),
    );
    let run = signed_in()
        .run(&[
            "check",
            "heartbeats",
            "HTTP check",
            "--device",
            DEVICE,
            "--limit",
            "1",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stderr.contains(&format!(
            "anima check heartbeats 'HTTP check' --device {DEVICE} --after h1.2"
        )),
        "{}",
        run.stderr
    );
}

#[tokio::test]
async fn cmd_026_s4_heartbeats_all_stops_when_the_hub_repeats_its_cursor() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CheckHeartbeats",
        beats("h1.2", true, "2026-11-12T10:00:00+00:00"),
    );
    let run = signed_in()
        .run(&["check", "heartbeats", CHECK, "--all", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("Stopped paging"), "{}", run.stderr);
}
