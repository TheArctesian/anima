//! `device watch`, `device metrics --watch`, `check watch` and `session watch` against the
//! scripted subscription hub (CMD-020, API-002).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::subscription_hub::{Script, SubscriptionHub};

const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
const CHECK: &str = "0190a000-0000-7000-8010-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-watch-tests")
}

/// Runs `anima <words> --hub <origin>` against a hub that plays `script`.
async fn watch(words: &[&str], script: Vec<Value>) -> (crate::kit::Outcome, SubscriptionHub) {
    let hub = SubscriptionHub::start(Script::Next(script)).await;
    let mut args = words.to_vec();
    args.extend(["--hub", &hub.origin]);
    let run = signed_in().run(&args).await;
    (run, hub)
}

#[tokio::test]
async fn cmd_020_device_watch_prints_one_line_for_each_status_change() {
    let changes = vec![
        json!({ "deviceStatusChanged": { "deviceId": DEVICE, "status": "DOWN", "at": "2026-11-13T08:00:00Z" } }),
        json!({ "deviceStatusChanged": { "deviceId": DEVICE, "status": "UP", "at": "2026-11-13T08:05:00Z" } }),
    ];
    let (run, hub) = watch(&["device", "watch", "--device", DEVICE], changes.clone()).await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{}", run.stdout);
    assert!(
        lines[0].contains("DOWN") && lines[0].contains(DEVICE),
        "{}",
        lines[0]
    );
    assert!(lines[1].contains("UP"), "{}", lines[1]);
    assert_eq!(hub.received().subscribe["variables"]["deviceId"], DEVICE);

    let (run, _) = watch(&["device", "watch", "--json"], changes).await;
    let first: Value =
        serde_json::from_str(run.stdout.lines().next().expect("a line")).expect("JSON");
    assert_eq!(first["status"], "DOWN");
    assert_eq!(first["deviceId"], DEVICE);
}

#[tokio::test]
async fn cmd_020_device_metrics_watch_prints_each_reading() {
    let readings = vec![json!({ "metricsAdded": {
        "deviceId": DEVICE, "at": "2026-11-13T08:00:00Z",
        "readings": { "cpuPercent": 12.34, "memoryPercent": null, "diskPercent": 60.0 },
    }})];
    let (run, hub) = watch(&["device", "metrics", DEVICE, "--watch"], readings).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("cpu 12.3%")
            && run.stdout.contains("memory -")
            && run.stdout.contains("disk 60.0%"),
        "{}",
        run.stdout
    );
    assert_eq!(hub.received().subscribe["variables"]["deviceId"], DEVICE);
}

#[tokio::test]
async fn cmd_020_check_watch_filters_by_check_and_device_and_prints_each_heartbeat() {
    let beats = vec![json!({ "heartbeatAdded": {
        "checkId": CHECK, "deviceId": DEVICE,
        "heartbeat": { "at": "2026-11-13T08:00:00Z", "status": "UP", "responseTimeMs": 12, "message": null },
    }})];
    let (run, hub) = watch(
        &["check", "watch", "--check", CHECK, "--device", DEVICE],
        beats.clone(),
    )
    .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("UP") && run.stdout.contains("12 ms"),
        "{}",
        run.stdout
    );
    let variables = &hub.received().subscribe["variables"];
    assert_eq!(variables["checkId"], CHECK);
    assert_eq!(variables["deviceId"], DEVICE);

    let (run, _) = watch(&["check", "watch", "--json"], beats).await;
    let first: Value = serde_json::from_str(run.stdout.trim()).expect("JSON");
    assert_eq!(first["heartbeat"]["responseTimeMs"], 12);
}

#[tokio::test]
async fn cmd_020_session_watch_prints_each_start_and_end() {
    let session = |ended: Value, reason: Value| {
        json!({ "sessionChanged": { "session": {
            "id": "0190a000-0000-7000-8040-000000000001", "deviceId": DEVICE, "deviceName": "pve-01",
            "protocol": "SSH", "operatorName": "stephen", "startedAt": "2026-11-13T08:00:00Z",
            "endedAt": ended, "endReason": reason, "clientAddr": "100.64.0.5",
        }}})
    };
    let changes = vec![
        session(Value::Null, Value::Null),
        session(json!("2026-11-13T08:10:00Z"), json!("OPERATOR")),
    ];
    let (run, _) = watch(&["session", "watch"], changes).await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{}", run.stdout);
    assert!(
        lines[0].contains("pve-01") && lines[0].contains("started"),
        "{}",
        lines[0]
    );
    assert!(lines[1].contains("ended (OPERATOR)"), "{}", lines[1]);
}

#[tokio::test]
async fn cmd_020_s3_a_hostile_time_in_a_metrics_reading_never_reaches_the_terminal_raw() {
    let readings = vec![json!({ "metricsAdded": {
        "deviceId": DEVICE, "at": "2026\u{1b}]0;pwned\u{7}\u{202E}-11-13",
        "readings": { "cpuPercent": 1.0, "memoryPercent": null, "diskPercent": null },
    }})];
    let (run, _) = watch(&["device", "metrics", DEVICE, "--watch"], readings).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        !run.stdout.contains(['\u{1b}', '\u{7}', '\u{202E}']),
        "{:?}",
        run.stdout
    );
    assert!(run.stdout.contains("cpu 1.0%"), "{}", run.stdout);
}
