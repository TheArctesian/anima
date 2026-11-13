//! A tool in state `ask` runs only when the person at the client approves the call through MCP
//! elicitation (MCP-013).

use serde_json::{Value, json};

use super::reads::{SESSION, session_row};
use super::session::{Desk, LOGIN_TOKEN, Session, assert_protocol_only, hub};
use crate::kit::fake_hub::FakeHub;
use crate::kit::fake_hub::data;

/// `desk` with saved permissions that ask before every change tool.
fn asking(hub: &FakeHub, desk: Desk) -> Desk {
    desk.save_permissions(&hub.origin, &json!({ "change": "ask" }));
    desk
}

/// A session whose client can ask the person.
async fn with_elicitation(hub: &FakeHub) -> Session {
    let desk = asking(hub, Desk::asking());
    desk.start(hub, LOGIN_TOKEN, &[]).await.0
}

fn text(answer: &Value) -> &str {
    answer["content"][0]["text"].as_str().unwrap_or_default()
}

fn operations(hub: &FakeHub) -> Vec<String> {
    hub.seen().into_iter().map(|seen| seen.operation).collect()
}

#[tokio::test]
async fn mcp_013_an_approved_call_reaches_the_hub_and_the_question_names_tool_and_arguments() {
    let hub = hub().await;
    hub.answer(
        "SessionEnd",
        data(json!({ "endSession": session_row(Some("OPERATOR")) })),
    );
    let mut session = with_elicitation(&hub).await;
    session.approval = Some(json!({ "action": "accept", "content": { "approve": true } }));
    let answer = session
        .call("anima_session_end", json!({ "session": SESSION }))
        .await;
    assert_eq!(answer["isError"], false, "{answer}");
    assert_eq!(
        answer["structuredContent"]["session"]["endReason"],
        "OPERATOR"
    );
    assert_eq!(session.asked.len(), 1);
    let message = session.asked[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("anima_session_end") && message.contains(SESSION),
        "{message}"
    );
    let schema = &session.asked[0]["requestedSchema"];
    assert_eq!(schema["properties"]["approve"]["type"], "boolean");
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, LOGIN_TOKEN);
    assert_eq!(operations(&hub), ["Me", "SessionEnd"]);
}

#[tokio::test]
async fn mcp_013_a_declined_or_unchecked_or_failed_question_refuses_without_the_hub() {
    let hub = hub().await;
    let mut session = with_elicitation(&hub).await;
    for approval in [
        Some(json!({ "action": "decline" })),
        Some(json!({ "action": "cancel" })),
        Some(json!({ "action": "accept", "content": { "approve": false } })),
        None,
    ] {
        session.approval = approval.clone();
        let answer = session
            .call("anima_session_end", json!({ "session": SESSION }))
            .await;
        assert_eq!(answer["isError"], true, "{approval:?}: {answer}");
        assert!(text(&answer).contains("ask the operator"), "{answer}");
        assert!(!text(&answer).contains("anima mcp"), "{answer}");
    }
    assert_eq!(session.asked.len(), 4);
    session.finish().await;
    assert_eq!(operations(&hub), ["Me"]);
}

#[tokio::test]
async fn mcp_013_a_client_without_elicitation_is_refused_and_sent_to_the_operator() {
    let hub = hub().await;
    let desk = asking(&hub, Desk::new());
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let names = session.tool_names().await;
    assert!(names.contains(&"anima_session_end".to_owned()), "listed");
    let answer = session
        .call("anima_session_end", json!({ "session": SESSION }))
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    let said = text(&answer);
    assert!(said.contains("elicitation"), "{said}");
    assert!(said.contains("hub was not contacted"), "{said}");
    assert_eq!(session.asked.len(), 0);
    session.finish().await;
    assert_eq!(operations(&hub), ["Me"]);
}

#[tokio::test]
async fn mcp_013_an_approved_delete_still_needs_its_confirm() {
    let hub = hub().await;
    let mut session = with_elicitation(&hub).await;
    session.approval = Some(json!({ "action": "accept", "content": { "approve": true } }));
    let answer = session
        .call(
            "anima_check_delete",
            json!({ "check": "SSH port", "device": "web-1", "confirm": "yes" }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    assert!(text(&answer).contains("Nothing was deleted"), "{answer}");
    session.finish().await;
    assert_eq!(operations(&hub), ["Me"]);
}

#[tokio::test]
async fn mcp_013_a_read_tool_in_allow_is_never_asked_about() {
    let hub = hub().await;
    hub.answer("RetentionList", data(json!({ "retention": [] })));
    let mut session = with_elicitation(&hub).await;
    let answer = session.call("anima_retention_list", json!({})).await;
    assert_eq!(answer["isError"], false, "{answer}");
    assert_eq!(session.asked.len(), 0);
    session.finish().await;
}

#[tokio::test]
async fn mcp_013_f1_the_question_shows_every_argument_after_long_values() {
    let hub = hub().await;
    let mut session = with_elicitation(&hub).await;
    session.approval = Some(json!({ "action": "decline" }));
    let long = "q".repeat(480);
    let answer = session
        .call(
            "anima_check_update",
            json!({ "check": "HTTPS", "device": "prod-db-01", "invert": true,
                    "config": { "http": { "url": "https://db.example/health",
                                           "keyword": long, "jsonQuery": long } } }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    let message = session.asked[0]["message"].as_str().unwrap_or_default();
    for shown in [
        "device: \"prod-db-01\"",
        "invert: true",
        "config.http.url: \"https://db.example/health\"",
        "config.http.jsonQuery:",
        "of 480 characters",
        "6 argument values",
    ] {
        assert!(message.contains(shown), "{shown} missing: {message}");
    }
    session.finish().await;
    assert_eq!(operations(&hub), ["Me"]);
}

#[tokio::test]
async fn mcp_013_f1_arguments_too_large_to_show_are_refused_without_asking() {
    let hub = hub().await;
    let mut session = with_elicitation(&hub).await;
    session.approval = Some(json!({ "action": "accept", "content": { "approve": true } }));
    let children: Vec<String> = (0..50).map(|index| format!("child-{index}")).collect();
    let answer = session
        .call(
            "anima_check_update",
            json!({ "check": "group", "device": "web-1",
                    "config": { "group": { "childIds": children } } }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    assert!(text(&answer).contains("Send a smaller change"), "{answer}");
    assert_eq!(session.asked.len(), 0);
    session.finish().await;
    assert_eq!(operations(&hub), ["Me"]);
}
