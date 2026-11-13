//! Who made each change, as the real hub records it (LOG-012..020): the operator's login token
//! is a User, an agent key and `anima mcp` with any token are an Agent, the key's name is kept,
//! `audit list --actor agent` gives only the agent's rows, and no row holds a token.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::hub::RealHub;
use super::keys::make_key;
use super::words;
use crate::mcp::session::Desk;

/// Every audit row on the hub, newest first, read by the hub's own query.
pub async fn audit_rows(hub: &RealHub, reader: &str) -> Vec<Value> {
    let query = "{ audit(first: 200) { edges { node { id action targetType targetId \
        operatorName actorKind tokenKind apiKeyName change } } } }";
    let data = hub.data(reader, query, json!({})).await;
    let edges = data["audit"]["edges"].as_array().expect("edges").iter();
    edges.map(|edge| edge["node"].clone()).collect()
}

/// The actor kind, token kind and key name of the one row about `target`.
async fn actor_of(hub: &RealHub, reader: &str, target: &str) -> (Value, Value, Value) {
    let rows = audit_rows(hub, reader).await;
    let about: Vec<&Value> = rows
        .iter()
        .filter(|row| row["targetId"] == target)
        .collect();
    assert_eq!(about.len(), 1, "one row about {target}: {rows:?}");
    let row = about[0];
    (
        row["actorKind"].clone(),
        row["tokenKind"].clone(),
        row["apiKeyName"].clone(),
    )
}

/// Creates the device `name` through `anima mcp` with `token`, the tool allowed in the saved
/// permissions; its ID.
async fn create_through_mcp(hub: &RealHub, token: &str, name: &str) -> String {
    let desk = Desk::on(hub.anima());
    desk.save_permissions(
        &hub.origin,
        &json!({ "tools": { "anima_device_create": "allow" } }),
    );
    let (mut session, _) = desk.start_at(&hub.origin, token, &[]).await;
    let arguments = json!({ "site": "home", "name": name, "host": "10.10.0.40",
                            "addDefaultCheck": false });
    let answer = session.call("anima_device_create", arguments).await;
    assert_eq!(answer["isError"], false, "{answer}");
    let id = answer["structuredContent"]["device"]["id"].as_str();
    let id = id
        .unwrap_or_else(|| panic!("a device id: {answer}"))
        .to_owned();
    session.finish().await;
    id
}

fn created_id(outcome: &crate::kit::Outcome) -> String {
    assert_eq!(outcome.code, 0, "{outcome:?}");
    outcome.json()["id"].as_str().expect("an id").to_owned()
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn log_012_each_change_is_recorded_with_who_made_it(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let login = hub.login_token().await;
    let operator = hub.anima().with("ANIMA_TOKEN", &login);
    let site = operator
        .run(&words("site create home --subnet 10.10.0.0/24 --json"))
        .await;
    let site = created_id(&site);
    let user_key = make_key(&operator, "script", "write", "user").await;
    let agent_key = make_key(&operator, "helper", "write", "agent").await;
    let user = (json!("USER"), json!("LOGIN"), Value::Null);
    assert_eq!(actor_of(&hub, &login, &site).await, user);

    let helper = hub.anima().with("ANIMA_TOKEN", &agent_key);
    let line = "device create by-key --site home --host 10.10.0.30 --no-default-check --json";
    let by_key = created_id(&helper.run(&words(line)).await);
    let agent = (json!("AGENT"), json!("API_KEY"), json!("helper"));
    assert_eq!(actor_of(&hub, &login, &by_key).await, agent);

    let by_login = create_through_mcp(&hub, &login, "by-mcp-login").await;
    let agent = (json!("AGENT"), json!("LOGIN"), Value::Null);
    assert_eq!(actor_of(&hub, &login, &by_login).await, agent);
    let by_user_key = create_through_mcp(&hub, &user_key, "by-mcp-key").await;
    let agent = (json!("AGENT"), json!("API_KEY"), json!("script"));
    assert_eq!(actor_of(&hub, &login, &by_user_key).await, agent);

    let listed = operator
        .run(&words("audit list --actor agent --json"))
        .await;
    assert_eq!(listed.code, 0, "{listed:?}");
    let listed = listed.json();
    let rows: Vec<&Value> = (listed["edges"].as_array())
        .unwrap_or_else(|| panic!("an audit connection: {listed}"))
        .iter()
        .map(|edge| &edge["node"])
        .collect();
    let mut targets: Vec<&str> = rows
        .iter()
        .filter_map(|row| row["targetId"].as_str())
        .collect();
    targets.sort_unstable();
    let mut expected = [by_key.as_str(), by_login.as_str(), by_user_key.as_str()];
    expected.sort_unstable();
    assert_eq!(targets, expected, "{listed}");
    assert!(
        rows.iter().all(|row| row["actorKind"] == "AGENT"),
        "{listed}"
    );

    let everything = json!(audit_rows(&hub, &login).await).to_string();
    for token in [&login, &user_key, &agent_key] {
        assert!(
            !everything.contains(token.as_str()),
            "a token in the audit log"
        );
    }
}
