//! `anima agents usage` against the scripted hub (AIS-020, AIS-021, AIS-024, CMD-020, CMD-030).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

const TOKEN: &str = "login-token-for-agent-usage-tests";
const MAC: &str = "0190a000-0000-7000-8002-00000000000a";
const LINUX: &str = "0190a000-0000-7000-8002-000000000009";

/// One K1 `AgentUsageRow` of `device` and `source`; `cost` null for no known price.
fn row(device: Option<(&str, &str)>, source: Option<&str>, cost: Option<f64>) -> Value {
    json!({
        "device": device.map(|(id, name)| json!({ "id": id, "name": name })),
        "source": source, "model": null, "repository": null, "day": null,
        "sessions": 3, "prompts": 12.0, "toolCalls": 41.0, "inputTokens": 1520.0,
        "outputTokens": 18_311.0, "cacheReadTokens": 2_210_450.0, "cacheWriteTokens": 90_112.0,
        "estimatedCostUsd": cost, "unpricedTokens": 300.0,
    })
}

/// The K1 `agentUsage` answer: a Mac with Claude Code and Codex, and a Linux PC whose daemon is
/// the system unit.
pub fn usage() -> Value {
    json!({ "agentUsage": {
        "firstDay": "2026-10-19", "lastDay": "2026-11-17", "rowsLeftOut": 0,
        "pricesAsOf": "2026-11-17",
        "rows": [
            row(Some((MAC, "mac-office-1")), Some("CLAUDE_CODE"), Some(31.554)),
            row(Some((MAC, "mac-office-1")), Some("CODEX"), None),
        ],
        "total": row(None, None, Some(31.554)),
        "devices": [
            { "device": { "id": LINUX, "name": "linux-pc-home" }, "state": "SYSTEM_UNIT" },
            { "device": { "id": MAC, "name": "mac-office-1" }, "state": "READING" }
        ],
        "accounts": [{
            "shortKey": "3f9a12c4", "source": "CODEX", "plan": "Pro",
            "reportedAt": "2026-11-17T10:00:00Z",
            "devices": [
                { "id": MAC, "name": "mac-office-1" },
                { "id": LINUX, "name": "linux-pc-home" }
            ],
            "limits": [
                { "window": "five_hour", "usedPercent": 35, "resetsAt": "2026-11-17T12:00:00Z",
                  "reached": false },
                { "window": "weekly", "usedPercent": null, "resetsAt": null, "reached": true }
            ]
        }]
    } })
}

#[tokio::test]
async fn ais_020_agents_usage_shows_each_group_the_total_and_what_the_estimate_is() {
    let hub = FakeHub::start().await;
    hub.answer("AgentUsage", data(usage()));
    let run = Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run(&["agents", "usage", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert!(lines[0].starts_with("DEVICE"), "{}", run.stdout);
    assert!(
        lines[0].contains("SOURCE") && lines[0].contains("EST. COST (USD)"),
        "{}",
        run.stdout
    );
    let claude = lines[1];
    for shown in [
        "mac-office-1",
        "Claude Code",
        "12",
        "41",
        "1.5k",
        "18.3k",
        "2.2M",
        "90.1k",
        "31.55",
    ] {
        assert!(claude.contains(shown), "{shown}: {}", run.stdout);
    }
    assert!(
        lines[2].contains("Codex") && lines[2].contains("no price"),
        "{}",
        run.stdout
    );
    assert!(lines[3].starts_with("total"), "{}", run.stdout);
    assert_eq!(
        lines[4..],
        [
            "Days 2026-10-19 to 2026-11-17 (UTC).",
            "The cost is an estimate from the public list prices of 2026-11-17; it is not the \
             bill of a subscription.",
            "300 tokens of models with no known price are not in the estimate.",
            "linux-pc-home: This daemon runs as a system unit and cannot see the sessions of a \
             user.",
            "Account 3f9a12c4 (Codex, Pro) on mac-office-1, linux-pc-home: five_hour 35 % used, \
             resets 2026-11-17T12:00:00Z; weekly reached.",
        ]
    );
    let sent = hub.seen().pop().expect("a request");
    assert_eq!(sent.variables["by"], json!(["DEVICE", "SOURCE"]));
    assert_eq!(
        sent.variables["filter"],
        json!({ "deviceId": null, "source": null, "repository": null, "firstDay": null,
                "lastDay": null })
    );
}

#[tokio::test]
async fn ais_020_agents_usage_sends_the_filter_and_days_and_prints_k1_json() {
    let hub = FakeHub::start().await;
    hub.answer("AgentUsage", data(usage()));
    let run = Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run(&[
            "agents",
            "usage",
            "--device",
            MAC,
            "--source",
            "codex",
            "--repository",
            "github.com/example/anima",
            "--days",
            "7",
            "--until",
            "2026-11-17",
            "--by",
            "model,day",
            "--by",
            "model",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), usage()["agentUsage"]);
    assert_eq!(run.stderr, "", "no note with --json");
    let sent = hub.seen().pop().expect("a request");
    assert_eq!(sent.variables["by"], json!(["MODEL", "DAY"]), "each once");
    assert_eq!(
        sent.variables["filter"],
        json!({ "deviceId": MAC, "source": "CODEX", "repository": "github.com/example/anima",
                "firstDay": "2026-11-11", "lastDay": "2026-11-17" })
    );
}

#[tokio::test]
async fn ais_020_agents_usage_refuses_days_with_since_or_a_bad_until() {
    let hub = FakeHub::start().await;
    for arguments in [
        ["--days", "3", "--since", "2026-11-01"],
        ["--days", "3", "--until", "17.11.2026"],
        ["--days", "0", "--until", "2026-11-17"],
    ] {
        let mut command = vec!["agents", "usage", "--hub", &hub.origin];
        command.extend(arguments);
        let run = Anima::new().with("ANIMA_TOKEN", TOKEN).run(&command).await;
        assert_eq!(run.code, 2, "{arguments:?}: {run:?}");
    }
    assert!(hub.seen().is_empty(), "nothing was asked of the hub");
}
