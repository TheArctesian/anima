//! `anima audit list` with who did each change: the actor kind, the token kind and the key name,
//! and the `--actor` filter (LOG-012, LOG-018, LOG-020). The command line never declares an
//! AI agent itself (LOG-016).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

const TOKEN: &str = "login-token-for-audit-tests";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

/// An audit node by `operator` with `who` (actor kind, token kind, key name).
fn node(id: u32, operator: &str, who: [Value; 3]) -> Value {
    let [actor, token, key] = who;
    json!({ "cursor": format!("a{id}"), "node": {
        "id": format!("0190a000-0000-7000-8031-{id:012}"), "at": "2026-11-13T08:00:00Z",
        "operatorName": operator, "action": "create", "targetType": "site",
        "targetId": "0190a000-0000-7000-8011-000000000001", "clientAddr": "10.10.0.9",
        "change": {}, "actorKind": actor, "tokenKind": token, "apiKeyName": key,
    }})
}

fn page(edges: &[Value]) -> Value {
    data(json!({ "audit": {
        "edges": edges, "pageInfo": { "hasNextPage": false, "endCursor": null },
    }}))
}

fn row<'a>(stdout: &'a str, operator: &str) -> &'a str {
    stdout
        .lines()
        .find(|line| line.contains(operator))
        .unwrap_or_else(|| panic!("no row for {operator} in {stdout}"))
}

#[tokio::test]
async fn log_020_audit_list_shows_the_actor_token_and_key_and_not_recorded_for_old_entries() {
    let hub = FakeHub::start().await;
    hub.answer(
        "AuditList",
        page(&[
            node(
                1,
                "ada",
                [json!("AGENT"), json!("API_KEY"), json!("claude")],
            ),
            node(2, "Anima", [json!("HUB"), Value::Null, Value::Null]),
            node(3, "old-ada", [Value::Null, Value::Null, Value::Null]),
        ]),
    );
    let run = signed_in()
        .run(&["audit", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let header = run.stdout.lines().next().expect("a header");
    for column in ["ACTOR", "TOKEN", "KEY"] {
        assert!(header.contains(column), "{header}");
    }
    let agent = row(&run.stdout, "ada ");
    assert!(
        agent.contains("AGENT") && agent.contains("API_KEY") && agent.contains("claude"),
        "{agent}"
    );
    assert!(row(&run.stdout, "Anima").contains("HUB"));
    assert!(row(&run.stdout, "old-ada").contains("Not recorded"));
    assert!(
        hub.seen().iter().all(|sent| sent.actor.is_none()),
        "the command line never declares an AI agent"
    );
}

#[tokio::test]
async fn log_020_audit_list_actor_filters_on_the_hub_and_keeps_it_for_the_next_page() {
    let hub = FakeHub::start().await;
    let mut answer = page(&[node(1, "ada", [json!("HUB"), Value::Null, Value::Null])]);
    answer["data"]["audit"]["pageInfo"] = json!({ "hasNextPage": true, "endCursor": "a1" });
    hub.answer("AuditList", answer);
    let run = signed_in()
        .run(&["audit", "list", "--actor", "hub", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let sent = hub.seen().pop().expect("a list");
    assert_eq!(sent.variables["actorKind"], "HUB");
    assert!(run.stderr.contains("--actor hub"), "{}", run.stderr);

    let wrong = signed_in()
        .run(&["audit", "list", "--actor", "robot", "--hub", &hub.origin])
        .await;
    assert_eq!(wrong.code, 2, "{wrong:?}");
}

/// An audit node of a change the hub sent to GitHub, with `record` as its change record.
fn sent(id: u32, operator: &str, record: &Value) -> Value {
    let mut entry = node(id, operator, [json!("USER"), json!("LOGIN"), Value::Null]);
    entry["node"]["targetType"] = json!("repository");
    entry["node"]["change"] = record.clone();
    entry
}

#[tokio::test]
async fn prj_028_audit_list_names_what_a_change_sent_to_github_did_and_to_what() {
    let hub = FakeHub::start().await;
    hub.answer(
        "AuditList",
        page(&[
            sent(
                1,
                "ada",
                &json!({ "function": "add_issue_comment", "repository": "acme/shop",
                    "target": "issue#60", "outcome": "done", "number": 60, "created": 7 }),
            ),
            sent(
                2,
                "bob",
                &json!({ "function": "merge_pull_request", "repository": "acme/shop",
                    "target": "pull#12", "outcome": "refused", "refusal": "conflict" }),
            ),
            node(3, "cy", [json!("USER"), json!("LOGIN"), Value::Null]),
        ]),
    );
    let run = signed_in()
        .run(&["audit", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let done = row(&run.stdout, "ada ");
    assert!(
        done.contains("add_issue_comment") && done.contains("acme/shop issue#60"),
        "{done}"
    );
    assert!(!done.contains("(done)"), "{done}");
    let refused = row(&run.stdout, "bob ");
    assert!(
        refused.contains("merge_pull_request (refused)") && refused.contains("acme/shop pull#12"),
        "{refused}"
    );
    let other = row(&run.stdout, "cy ");
    assert!(
        other.contains("create") && other.contains("site 0190a000-0000-7000-8011-000000000001"),
        "{other}"
    );
}

/// A K1 `AuditChainCheck`: whole with five links, or broken at link 3 by a changed entry.
fn chain_check(broken: bool) -> Value {
    let fault = broken.then(|| {
        json!({ "kind": "CHANGED", "seq": 3,
                "entryId": "0190a000-0000-7000-8031-000000000003", "at": "2026-11-13T08:00:00Z" })
    });
    json!({ "auditChain": {
        "intact": !broken, "links": 5, "dropped": 0, "headSeq": 5,
        "headHash": "ab".repeat(32), "fault": fault,
    }})
}

#[tokio::test]
async fn log_027_audit_verify_says_whole_or_broken_with_the_first_fault() {
    for broken in [false, true] {
        let hub = FakeHub::start().await;
        hub.answer("AuditChain", data(chain_check(broken)));
        let run = signed_in()
            .run(&["audit", "verify", "--hub", &hub.origin])
            .await;
        assert_eq!(run.code, 0, "{run:?}");
        let word = if broken { "broken" } else { "whole" };
        assert!(run.stdout.contains(word), "{run:?}");
        assert!(run.stdout.contains(&"ab".repeat(32)), "the head: {run:?}");
        assert_eq!(run.stdout.contains("CHANGED at link 3"), broken, "{run:?}");
    }
}
