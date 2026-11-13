//! `release log --follow` and `vpn status --watch` against the scripted subscription hub
//! (CMD-020, BLD-081).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::subscription_hub::{Script, SubscriptionHub};

const JOB: &str = "0190a000-0000-7000-8060-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-watch-tests")
}

fn update(state: &str, reason: Option<&str>, lines: &[(i64, &str)]) -> Value {
    let log_lines: Vec<Value> = lines
        .iter()
        .map(|(sequence, text)| json!({ "sequence": sequence, "text": text }))
        .collect();
    json!({ "buildJobUpdated": {
        "job": { "id": JOB, "state": state, "failureReason": reason, "stateChangedAt": "2026-11-13T08:00:00Z" },
        "logLines": log_lines,
    }})
}

#[tokio::test]
async fn bld_081_release_log_follow_prints_new_lines_and_stops_at_an_end_state() {
    let updates = vec![
        update(
            "BUILDING",
            Value::Null,
            &[(1, "cargo build"), (2, "Sess\u{1b}[2Jion")],
        ),
        update("FAILED", Some("exit 101"), &[(3, "error: linking")]),
        update("BUILDING", None, &[(4, "never printed")]),
    ];
    let hub = SubscriptionHub::start(Script::Next(updates)).await;
    let run = signed_in()
        .run(&["release", "log", JOB, "--follow", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines, ["cargo build", "Sess [2Jion", "error: linking"]);
    assert!(
        run.stderr.contains("FAILED") && run.stderr.contains("exit 101"),
        "{}",
        run.stderr
    );
    assert_eq!(hub.received().subscribe["variables"]["jobId"], JOB);
}

#[tokio::test]
async fn bld_081_release_log_follow_json_is_one_update_per_line() {
    let updates = vec![update("PUBLISHED", None, &[(1, "done")])];
    let hub = SubscriptionHub::start(Script::Next(updates)).await;
    let run = signed_in()
        .run(&[
            "release",
            "log",
            JOB,
            "--follow",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let first: Value = serde_json::from_str(run.stdout.trim()).expect("JSON");
    assert_eq!(first["job"]["state"], "PUBLISHED");
    assert_eq!(first["logLines"][0]["text"], "done");
}

#[tokio::test]
async fn cmd_020_vpn_status_watch_prints_one_line_for_each_picture() {
    let picture = json!({ "vpnChanged": {
        "readAt": "2026-11-13T08:00:00Z", "fresh": true, "broker": "REACHABLE",
        "sites": [{ "site": { "name": "Home" }, "state": "UP", "path": "DIRECT" }],
        "nodes": [{ "name": "hub", "online": true }, { "name": "nas", "online": false }],
    }});
    let hub = SubscriptionHub::start(Script::Next(vec![picture])).await;
    let run = signed_in()
        .run(&["vpn", "status", "--watch", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 1, "{}", run.stdout);
    assert!(
        lines[0].contains("fresh")
            && lines[0].contains("nodes 1/2 online")
            && lines[0].contains("Home UP DIRECT"),
        "{}",
        lines[0]
    );
}
