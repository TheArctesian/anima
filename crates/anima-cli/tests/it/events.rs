//! `anima event list`, `audit list` and `event watch` against the scripted hubs (CMD-020,
//! CMD-026, API-002).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};
use crate::kit::subscription_hub::{Script, SubscriptionHub};

const TOKEN: &str = "login-token-for-event-tests";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

fn event(id: u32, severity: &str, device: Option<&str>, message: &str) -> Value {
    json!({
        "id": format!("0190a000-0000-7000-8030-{id:012}"),
        "at": format!("2026-11-06T08:0{id}:00Z"),
        "severity": severity, "source": "CHECK",
        "deviceId": device.map(|_| "0190a000-0000-7000-8002-000000000001"),
        "deviceName": device, "message": message,
    })
}

fn events_page(nodes: &[Value], next: Option<&str>) -> Value {
    let edges: Vec<Value> = nodes
        .iter()
        .map(|node| json!({ "cursor": format!("c-{}", node["id"]), "node": node }))
        .collect();
    data(json!({ "events": {
        "edges": edges,
        "pageInfo": { "hasNextPage": next.is_some(), "endCursor": next },
    }}))
}

#[tokio::test]
async fn cmd_020_event_list_prints_a_table_and_sends_the_filter() {
    let hub = FakeHub::start().await;
    let nodes = [
        event(1, "ERROR", Some("pve-01"), "Ping is down."),
        event(2, "INFO", None, "The hub started."),
    ];
    hub.answer("EventList", events_page(&nodes, None));
    let device = "0190a000-0000-7000-8002-000000000001";
    let run = signed_in()
        .run(&[
            "event",
            "list",
            "--severity",
            "error",
            "--severity",
            "warning",
            "--source",
            "check",
            "--device",
            device,
            "--since",
            "2026-11-01T00:00:00Z",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let header = run.stdout.lines().next().expect("a header");
    assert!(header.starts_with("AT") && header.contains("SEVERITY") && header.contains("MESSAGE"));
    let row = run
        .stdout
        .lines()
        .find(|line| line.contains("pve-01"))
        .expect("a row");
    assert!(row.contains("ERROR") && row.contains("CHECK") && row.contains("Ping is down."));
    assert!(run.stdout.contains("The hub started."), "{}", run.stdout);
    let seen = hub.seen();
    assert_eq!(
        seen[0].variables["filter"],
        json!({ "severity": ["ERROR", "WARNING"], "source": ["CHECK"],
                "deviceId": device, "since": "2026-11-01T00:00:00Z" })
    );
    assert_eq!(seen[0].variables["first"], 100);
    assert_eq!(
        seen[0].authorization.as_deref(),
        Some("Bearer login-token-for-event-tests")
    );
    run.assert_never_shows(TOKEN);
}

#[tokio::test]
async fn cmd_026_event_list_follows_pages_with_all_and_names_the_next_cursor_without_it() {
    let hub = FakeHub::start().await;
    let first = [event(1, "INFO", None, "one")];
    let second = [event(2, "INFO", None, "two")];
    hub.answer("EventList", events_page(&first, Some("cursor-1")))
        .answer("EventList", events_page(&second, None));
    let run = signed_in()
        .run(&["event", "list", "--all", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let json = run.json();
    assert_eq!(json["edges"].as_array().expect("edges").len(), 2);
    assert_eq!(json["pageInfo"]["hasNextPage"], false);
    let seen = hub.seen();
    assert_eq!(seen[0].variables["first"], 200);
    assert_eq!(seen[1].variables["after"], "cursor-1");

    let paged = FakeHub::start().await;
    paged.answer("EventList", events_page(&first, Some("cursor-9")));
    let run = signed_in()
        .run(&["event", "list", "--limit", "1", "--hub", &paged.origin])
        .await;
    assert!(run.stderr.contains("--after cursor-9"), "{}", run.stderr);
}

#[tokio::test]
async fn cmd_021_event_list_with_a_refused_token_is_exit_4() {
    let hub = FakeHub::start().await;
    hub.answer("EventList", refusal("UNAUTHENTICATED", "Sign in first."));
    let run = signed_in()
        .run(&["event", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 4, "{run:?}");
    assert_eq!(run.error()["code"], "UNAUTHENTICATED");
}

#[tokio::test]
async fn cmd_020_audit_list_shows_who_did_what_and_json_keeps_the_change_record() {
    let hub = FakeHub::start().await;
    let node = json!({
        "id": "0190a000-0000-7000-8031-000000000001", "at": "2026-11-06T08:00:00Z",
        "operatorName": "ada", "action": "device.update", "targetType": "device",
        "targetId": "0190a000-0000-7000-8002-000000000001", "clientAddr": "10.10.0.9",
        "change": { "name": ["old", "new"], "password": "[redacted]" },
    });
    hub.answer(
        "AuditList",
        data(json!({ "audit": {
            "edges": [{ "cursor": "a1", "node": node }],
            "pageInfo": { "hasNextPage": false, "endCursor": "a1" },
        }})),
    );
    let run = signed_in()
        .run(&["audit", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run
        .stdout
        .lines()
        .find(|line| line.contains("ada"))
        .expect("a row");
    assert!(
        row.contains("device.update") && row.contains("10.10.0.9"),
        "{row}"
    );

    let run = signed_in()
        .run(&["audit", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(
        run.json()["edges"][0]["node"]["change"]["password"],
        "[redacted]"
    );
}

#[tokio::test]
async fn api_002_event_watch_signs_in_in_the_payload_and_prints_one_line_for_each_event() {
    let events = vec![
        json!({ "eventAdded": event(1, "ERROR", Some("pve-01"), "Ping is down.") }),
        json!({ "eventAdded": event(2, "INFO", None, "Sess\u{1b}[2Jion opened.") }),
    ];
    let hub = SubscriptionHub::start(Script::Next(events)).await;
    let run = signed_in()
        .run(&[
            "event",
            "watch",
            "--min-severity",
            "warning",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{}", run.stdout);
    assert!(
        lines[0].contains("ERROR") && lines[0].contains("pve-01"),
        "{}",
        lines[0]
    );
    assert!(
        lines[1].contains("Sess [2Jion opened."),
        "control characters are cleaned: {}",
        lines[1]
    );

    let received = hub.received();
    assert_eq!(
        received.init,
        json!({ "authorization": "Bearer login-token-for-event-tests" })
    );
    assert!(
        !received.uri.contains(TOKEN),
        "the token is never in the URL: {}",
        received.uri
    );
    assert_eq!(received.uri, "/graphql");
    assert_eq!(received.pongs, 1, "the client answers the hub's ping");
    assert_eq!(received.subscribe["variables"]["minSeverity"], "WARNING");
    assert!(
        received.subscribe["query"]
            .as_str()
            .is_some_and(|query| query.contains("eventAdded")),
        "{}",
        received.subscribe
    );
    run.assert_never_shows(TOKEN);
}

#[tokio::test]
async fn cmd_020_event_watch_json_is_one_k1_event_per_line() {
    let events = vec![
        json!({ "eventAdded": event(1, "WARNING", Some("pve-01"), "Slow.") }),
        json!({ "eventAdded": event(2, "ERROR", None, "Down.") }),
    ];
    let hub = SubscriptionHub::start(Script::Next(events)).await;
    let run = signed_in()
        .run(&["event", "watch", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let parsed: Vec<Value> = run
        .stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["severity"], "WARNING");
    assert_eq!(parsed[0]["deviceName"], "pve-01");
    assert_eq!(parsed[1]["message"], "Down.");
    assert!(hub.received().subscribe["variables"]["minSeverity"].is_null());
}

#[tokio::test]
async fn cmd_021_event_watch_with_a_token_the_hub_closes_on_with_4403_is_exit_4() {
    let hub = SubscriptionHub::start(Script::Refuse).await;
    let run = signed_in()
        .run(&["event", "watch", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 4, "{run:?}");
    assert_eq!(run.error()["code"], "UNAUTHENTICATED");
    assert_eq!(run.stdout, "");
    run.assert_never_shows(TOKEN);
}

#[tokio::test]
async fn cmd_021_event_watch_ends_with_the_code_of_an_error_frame() {
    let errors = json!([{ "message": "Not permitted.", "extensions": { "code": "FORBIDDEN" } }]);
    let hub = SubscriptionHub::start(Script::Error(errors)).await;
    let run = signed_in()
        .run(&["event", "watch", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    assert!(run.stderr.contains("Not permitted."), "{}", run.stderr);
}
