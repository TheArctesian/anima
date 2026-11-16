//! The issue change tools refuse text over the hub's limits before any request, in fixed words
//! (MCP-008, PRJ-028).

use serde_json::json;

use super::super::session::{API_KEY, Desk, hub};
use crate::kit::forge::{FULL_NAME, project};

#[tokio::test]
async fn mcp_008_an_issue_or_comment_over_the_hubs_limits_is_refused_before_any_request() {
    let hub = hub().await;
    project(&hub);
    let mut session = Desk::new().start(&hub, API_KEY, &["--allow-write"]).await.0;
    let long_title = "t".repeat(257);
    let long_body = "b".repeat(65_537);
    let long_label = "l".repeat(51);
    let many_labels: Vec<String> = (0..21).map(|n| format!("label{n}")).collect();
    let calls = [
        (
            "anima_issue_create",
            json!({ "repository": FULL_NAME, "title": "" }),
        ),
        (
            "anima_issue_create",
            json!({ "repository": FULL_NAME, "title": long_title }),
        ),
        (
            "anima_issue_create",
            json!({ "repository": FULL_NAME, "title": "ok", "body": long_body }),
        ),
        (
            "anima_issue_create",
            json!({ "repository": FULL_NAME, "title": "ok", "labels": [long_label] }),
        ),
        (
            "anima_issue_create",
            json!({ "repository": FULL_NAME, "title": "ok", "labels": many_labels }),
        ),
        (
            "anima_issue_comment_add",
            json!({ "repository": FULL_NAME, "number": 12, "body": "" }),
        ),
        (
            "anima_issue_comment_add",
            json!({ "repository": FULL_NAME, "number": 12, "body": long_body }),
        ),
    ];
    for (tool, arguments) in calls {
        let refused = session.call(tool, arguments).await;
        assert_eq!(refused["isError"], true, "{tool}: {refused}");
        let words = refused["content"][0]["text"].as_str().unwrap_or_default();
        assert!(words.starts_with("Nothing was sent:"), "{tool}: {words}");
        assert!(
            !words.contains("ttttt") && !words.contains("bbbbb"),
            "{words}"
        );
    }
    session.finish().await;
    let operations: Vec<String> = hub.seen().into_iter().map(|seen| seen.operation).collect();
    assert_eq!(operations, ["Me"]);
}
