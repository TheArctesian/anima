//! A tool in state `ask` runs only when the person at the client approves the call through MCP
//! elicitation (MCP-013).

use serde_json::{Value, json};

use super::reads::{SESSION, session_row};
use super::session::{Desk, LOGIN_TOKEN, Session, assert_protocol_only, hub};
use crate::kit::fake_hub::FakeHub;
use crate::kit::fake_hub::data;

/// `desk` with saved permissions that ask before every change tool.
async fn asking(hub: &FakeHub, desk: Desk) -> Desk {
    let (code, _, stderr) = desk
        .run(
            LOGIN_TOKEN,
            &[
                "mcp",
                "permissions",
                "set",
                "--ask-all",
                "--kind",
                "change",
                "--hub",
                &hub.origin,
            ],
        )
        .await;
    assert_eq!(code, 0, "{stderr}");
    desk
}

/// A session whose client can ask the person.
async fn with_elicitation(hub: &FakeHub) -> Session {
    let desk = asking(hub, Desk::asking()).await;
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
        assert!(
            text(&answer).contains("anima mcp permissions set --allow anima_session_end"),
            "{answer}"
        );
    }
    assert_eq!(session.asked.len(), 4);
    session.finish().await;
    assert_eq!(operations(&hub), ["Me"]);
}

#[tokio::test]
async fn mcp_013_a_client_without_elicitation_is_refused_with_the_way_to_allow_the_tool() {
    let hub = hub().await;
    let desk = asking(&hub, Desk::new()).await;
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
