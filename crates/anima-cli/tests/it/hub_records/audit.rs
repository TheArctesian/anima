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
