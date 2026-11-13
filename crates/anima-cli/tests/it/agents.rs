//! `anima agent` against the scripted hub (CMD-020, CMD-021, CMD-024).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-agent-tests";
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
async fn cmd_020_agent_status_says_whether_the_agent_is_reporting() {
    let hub = FakeHub::start().await;
    let device = |stale: bool| {
        data(
            json!({ "device": { "id": DEVICE, "name": "nas", "metricsSource": "AGENT",
            "metricsStale": stale, "readings": { "at": "2026-11-13T09:00:00+00:00" } } }),
        )
    };
    hub.answer("AgentStatus", device(false))
        .answer("AgentStatus", device(true));
    let run = signed_in()
        .run(&["agent", "status", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("reporting") && run.stdout.contains("2026-11-13T09:00:00"),
        "{}",
        run.stdout
    );

    let run = signed_in()
        .run(&["agent", "status", DEVICE, "--hub", &hub.origin])
        .await;
    assert!(run.stdout.contains("stale"), "{}", run.stdout);

    let run = signed_in()
        .run(&["agent", "status", DEVICE, "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["metricsSource"], "AGENT");
}

#[tokio::test]
async fn met_012_agent_enrol_prints_the_one_time_code_and_its_expiry() {
    let hub = FakeHub::start().await;
    hub.answer(
        "AgentEnrol",
        data(json!({ "enrolAgent": { "code": CODE, "expiresAt": "2026-11-13T09:15:00+00:00" } })),
    );
    let run = signed_in()
        .run(&["agent", "enrol", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let code_line = run
        .stdout
        .lines()
        .find(|line| line.starts_with("Code"))
        .expect("a code line");
    assert!(code_line.ends_with(CODE), "{code_line}");
    assert!(run.stdout.contains("2026-11-13T09:15:00"));
    assert!(
        !run.stderr.contains(CODE),
        "the code is printed once, on standard output"
    );
    assert_eq!(variables(&hub, "AgentEnrol")["deviceId"], DEVICE);

    let run = signed_in()
        .run(&["agent", "enrol", DEVICE, "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["code"], CODE);
    assert_eq!(run.json()["expiresAt"], "2026-11-13T09:15:00+00:00");
}

#[tokio::test]
async fn cmd_021_an_api_key_cannot_enrol_an_agent() {
    let hub = FakeHub::start().await;
    hub.answer(
        "AgentEnrol",
        refusal("FORBIDDEN", "Only a login token may do this."),
    );
    let run = signed_in()
        .run(&["agent", "enrol", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    run.assert_never_shows(CODE);
}

#[tokio::test]
async fn cmd_024_agent_revoke_needs_yes_and_prints_the_stale_source() {
    let hub = FakeHub::start().await;
    hub.answer(
        "AgentRevoke",
        data(json!({ "revokeAgentToken": { "id": DEVICE, "name": "nas",
            "metricsSource": "AGENT", "metricsStale": true } })),
    );
    let run = signed_in()
        .run(&["agent", "revoke", DEVICE, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(hub.seen().is_empty());

    let run = signed_in()
        .run(&["agent", "revoke", DEVICE, "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("AGENT") && run.stdout.contains("yes"),
        "{}",
        run.stdout
    );
    assert_eq!(variables(&hub, "AgentRevoke")["deviceId"], DEVICE);
}
