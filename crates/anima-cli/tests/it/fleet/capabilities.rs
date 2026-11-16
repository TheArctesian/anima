//! `anima capability list` against the scripted hub (CAP-030, CMD-020, CMD-030).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

const TOKEN: &str = "login-token-for-capability-tests";

/// The K1 `capabilities` answer: a Mac with cargo and a failed bun, and a Linux PC with nothing.
pub fn capabilities() -> Value {
    json!({ "capabilities": [
        { "device": { "id": "0190a000-0000-7000-8002-00000000000a", "name": "mac-office-1" },
          "architecture": "aarch64", "os": "macos", "osVersion": "27.0",
          "tools": [ { "name": "cargo", "version": "1.91.0", "usable": true },
                     { "name": "bun", "version": null, "usable": false } ],
          "buildTargets": ["aarch64-apple-darwin", "aarch64-apple-ios"],
          "foundAt": "2026-11-15T09:00:00+00:00" },
        { "device": { "id": "0190a000-0000-7000-8002-000000000009", "name": "linux-pc-home" },
          "architecture": "x86_64", "os": "linux", "osVersion": "24.04",
          "tools": [], "buildTargets": [], "foundAt": "2026-11-15T09:01:00+00:00" }
    ] })
}

#[tokio::test]
async fn cap_030_capability_list_shows_each_device_s_system_tools_and_targets() {
    let hub = FakeHub::start().await;
    hub.answer("CapabilityList", data(capabilities()));
    let run = Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run(&["capability", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert!(lines[0].starts_with("DEVICE"), "{}", run.stdout);
    assert!(
        lines[1].contains("mac-office-1")
            && lines[1].contains("macos 27.0 aarch64")
            && lines[1].contains("cargo 1.91.0, bun not usable")
            && lines[1].contains("aarch64-apple-darwin, aarch64-apple-ios"),
        "{}",
        run.stdout
    );
    assert!(
        lines[2].contains("linux-pc-home") && lines[2].contains("linux 24.04 x86_64"),
        "{}",
        run.stdout
    );
    let sent = hub.seen().pop().expect("a request");
    assert_eq!(sent.variables["tool"], Value::Null);
    assert_eq!(sent.variables["buildTarget"], Value::Null);
}

#[tokio::test]
async fn cap_030_capability_list_filters_by_tool_and_target_and_prints_k1_json() {
    let hub = FakeHub::start().await;
    hub.answer("CapabilityList", data(capabilities()));
    let run = Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run(&[
            "capability",
            "list",
            "--tool",
            "cargo",
            "--target",
            "aarch64-apple-darwin",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), capabilities()["capabilities"]);
    let sent = hub.seen().pop().expect("a request");
    assert_eq!(sent.variables["tool"], "cargo");
    assert_eq!(sent.variables["buildTarget"], "aarch64-apple-darwin");
}
