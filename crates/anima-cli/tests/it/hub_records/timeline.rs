//! `anima timeline list` against the scripted hub (TML-002, TML-005 to TML-007, CMD-026).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-timeline-tests")
}

fn entry(reference: &str, title: &str, superseded_by: Option<&str>) -> Value {
    json!({
        "reference": reference, "source": "event_log", "category": "DEVICE", "title": title,
        "start": "2026-11-06T08:00:00+00:00", "end": null, "basis": "OBSERVED",
        "uncertainty": ["End time not given by the source."], "supersededBy": superseded_by,
    })
}

fn page(nodes: &[Value], next: Option<&str>) -> Value {
    let edges: Vec<Value> = nodes
        .iter()
        .map(|node| json!({ "cursor": format!("c-{}", node["reference"]), "node": node }))
        .collect();
    data(json!({ "timeline": {
        "edges": edges,
        "pageInfo": { "hasNextPage": next.is_some(), "endCursor": next },
    }}))
}

#[tokio::test]
async fn tml_007_timeline_list_shows_each_entry_with_its_uncertainty_and_sends_the_span() {
    let hub = FakeHub::start().await;
    let nodes = [
        entry("event:1", "pve-01: Ping is down.", None),
        entry("event:2", "A second record.", Some("event:1")),
    ];
    hub.answer("TimelineList", page(&nodes, None));
    let run = signed_in()
        .run(&[
            "timeline",
            "list",
            "--from",
            "2026-11-06T08:00:00+08:00",
            "--to",
            "2026-11-07T00:00:00Z",
            "--category",
            "device",
            "--category",
            "session",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let header = run.stdout.lines().next().expect("a header");
    assert!(header.starts_with("START") && header.contains("CATEGORY") && header.contains("NOTES"));
    let row = run
        .stdout
        .lines()
        .find(|line| line.contains("pve-01"))
        .expect("a row");
    assert!(row.contains("DEVICE") && row.contains("OBSERVED") && row.contains("event:1"));
    assert!(row.contains("End time not given by the source."), "{row}");
    let second = run
        .stdout
        .lines()
        .find(|line| line.contains("A second record."))
        .expect("the superseded entry is listed too");
    assert!(second.contains("Superseded by event:1."), "{second}");

    let seen = hub.seen();
    assert_eq!(seen[0].variables["from"], "2026-11-06T00:00:00.000000Z");
    assert_eq!(seen[0].variables["to"], "2026-11-07T00:00:00.000000Z");
    assert_eq!(
        seen[0].variables["categories"],
        json!(["DEVICE", "SESSION"])
    );
    assert_eq!(seen[0].variables["first"], 100);
}

#[tokio::test]
async fn tml_007_with_no_time_the_span_is_one_day_and_json_is_the_k1_connection() {
    let hub = FakeHub::start().await;
    hub.answer("TimelineList", page(&[entry("event:1", "one", None)], None));
    let run = signed_in()
        .run(&["timeline", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        run.json()["edges"][0]["node"],
        entry("event:1", "one", None)
    );
    let seen = hub.seen();
    let instant = |name: &str| {
        let text = seen[0].variables[name].as_str().expect("a time").to_owned();
        chrono::DateTime::parse_from_rfc3339(&text).expect("RFC 3339")
    };
    assert_eq!(instant("to") - instant("from"), chrono::Duration::days(1));
    assert!(
        seen[0]
            .variables
            .get("categories")
            .is_none_or(Value::is_null)
    );
}

#[tokio::test]
async fn cmd_026_timeline_list_follows_pages_with_all_and_names_the_next_cursor_without_it() {
    let hub = FakeHub::start().await;
    hub.answer(
        "TimelineList",
        page(&[entry("event:1", "one", None)], Some("cursor-1")),
    )
    .answer("TimelineList", page(&[entry("event:2", "two", None)], None));
    let run = signed_in()
        .run(&["timeline", "list", "--all", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["edges"].as_array().expect("edges").len(), 2);
    let seen = hub.seen();
    assert_eq!(seen[0].variables["first"], 200);
    assert_eq!(seen[1].variables["after"], "cursor-1");
    assert_eq!(seen[1].variables["to"], seen[0].variables["to"]);

    let paged = FakeHub::start().await;
    paged.answer(
        "TimelineList",
        page(&[entry("event:1", "one", None)], Some("cursor-9")),
    );
    let run = signed_in()
        .run(&[
            "timeline",
            "list",
            "--to",
            "2026-11-07T00:00:00Z",
            "--limit",
            "1",
            "--hub",
            &paged.origin,
        ])
        .await;
    assert!(run.stderr.contains("--after cursor-9"), "{}", run.stderr);
    assert!(
        run.stderr.contains("--to 2026-11-07T00:00:00.000000Z"),
        "{}",
        run.stderr
    );
}

#[tokio::test]
async fn tml_007_a_time_that_is_not_rfc_3339_is_a_usage_error_before_the_hub() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&[
            "timeline",
            "list",
            "--from",
            "yesterday",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_ne!(run.code, 0, "{run:?}");
    assert!(run.stderr.contains("--from is a time"), "{}", run.stderr);
    assert!(hub.seen().is_empty());

    hub.answer(
        "TimelineList",
        refusal("VALIDATION", "Some input is not valid."),
    );
    let run = signed_in()
        .run(&[
            "timeline",
            "list",
            "--after",
            "x",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.error()["code"], "VALIDATION", "{run:?}");
}
