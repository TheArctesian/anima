//! `anima session list` and `anima session end` (CMD-020, CMD-026).

use serde_json::json;

use super::{LOGIN_TOKEN, SESSION, ended, session};
use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", LOGIN_TOKEN)
}

fn page(next: Option<&str>) -> serde_json::Value {
    data(json!({ "sessions": {
        "edges": [
            { "cursor": "c-1", "node": session(None) },
            { "cursor": "c-2", "node": session(Some("HOST_KEY_MISMATCH")) },
        ],
        "pageInfo": { "hasNextPage": next.is_some(), "endCursor": next },
    }}))
}

#[tokio::test]
async fn cmd_020_session_list_prints_live_and_ended_sessions() {
    let hub = FakeHub::start().await;
    hub.answer("SessionList", page(Some("c-2")));
    let run = signed_in()
        .run(&["session", "list", "--live", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let header = run.stdout.lines().next().unwrap_or_default();
    assert!(header.starts_with("DEVICE"), "{run:?}");
    assert!(run.stdout.contains("live"), "{run:?}");
    assert!(run.stdout.contains("HOST_KEY_MISMATCH"), "{run:?}");
    assert!(run.stderr.contains("--after c-2"), "{run:?}");
    assert_eq!(hub.seen()[0].variables["live"], true);
}

#[tokio::test]
async fn cmd_020_session_list_json_keeps_the_k1_field_names() {
    let hub = FakeHub::start().await;
    hub.answer("SessionList", page(None));
    let run = signed_in()
        .run(&["session", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let sessions = run.json();
    assert_eq!(
        sessions["edges"][1]["node"]["endReason"],
        "HOST_KEY_MISMATCH"
    );
    assert_eq!(sessions["edges"][0]["node"]["clientAddr"], "10.10.0.5");
}

#[tokio::test]
async fn cmd_026_all_follows_every_page() {
    let hub = FakeHub::start().await;
    hub.answer("SessionList", page(Some("c-2")))
        .answer("SessionList", page(None));
    let run = signed_in()
        .run(&["session", "list", "--all", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["edges"].as_array().map(Vec::len), Some(4));
    assert_eq!(hub.seen()[1].variables["after"], "c-2");
}

#[tokio::test]
async fn session_end_prints_the_ended_session() {
    let hub = FakeHub::start().await;
    hub.answer("SessionEnd", ended("OPERATOR"));
    let run = signed_in()
        .run(&["session", "end", SESSION, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("OPERATOR"), "{run:?}");
    assert_eq!(hub.seen()[0].variables["id"], SESSION);
}

#[tokio::test]
async fn session_end_refuses_what_is_not_an_id_and_keeps_the_hubs_codes() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&["session", "end", "web-1", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(hub.seen().is_empty());
    hub.answer("SessionEnd", refusal("NOT_FOUND", "No such session."));
    let run = signed_in()
        .run(&["session", "end", SESSION, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
}

#[tokio::test]
async fn cmd_026_s4_all_stops_when_the_hub_repeats_its_cursor() {
    let hub = FakeHub::start().await;
    hub.answer("SessionList", page(Some("c-2")));
    let run = signed_in()
        .run(&["session", "list", "--all", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("Stopped paging"), "{}", run.stderr);
    assert_eq!(hub.seen().len(), 2, "one page, then the repeat");
}

#[tokio::test]
async fn cmd_026_c3_the_next_page_hint_keeps_the_filters() {
    let hub = FakeHub::start().await;
    hub.answer("SessionList", page(Some("c-2")));
    let device = "0190a000-0000-7000-8002-000000000001";
    let run = signed_in()
        .run(&[
            "session",
            "list",
            "--live",
            "--device",
            device,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert!(
        run.stderr.contains(&format!(
            "anima session list --device {device} --live --after c-2"
        )),
        "{}",
        run.stderr
    );
}
