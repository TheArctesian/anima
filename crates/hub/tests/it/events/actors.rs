//! Who did it: the actor kind, token kind and key name of each entry, decided from the request
//! through the hub's router (LOG-012 to LOG-018, LOG-020).

use std::sync::atomic::{AtomicU8, Ordering};

use axum::{
    body::Body,
    http::{Request, header},
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use anima_hub::{access, events::ActorKind};

use crate::hub::{Hub, set_operator, start};

const CREATE_KEY: &str = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { \
    secret key { name madeFor } } }";

const AUDIT: &str = "query($kind: ActorKind) { audit(first: 50, actorKind: $kind) { edges { \
    node { operatorName action targetType actorKind tokenKind apiKeyName } } } }";

const EVENTS: &str = "{ events(first: 50) { edges { node { message actorKind } } } }";

/// Posts `query` with `token` and, when `declared`, the header `Anima-Actor: agent`.
async fn post(
    hub: &Hub,
    token: &str,
    (query, variables): (&str, Value),
    declared: Option<&str>,
) -> Value {
    let body = json!({ "query": query, "variables": variables }).to_string();
    let mut request = Request::post("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    if let Some(value) = declared {
        request = request.header("Anima-Actor", value);
    }
    let (status, text) = hub.send(request.body(Body::from(body)).unwrap()).await;
    assert!(status.is_success(), "{status} {text}");
    serde_json::from_str(&text).unwrap()
}

/// A change any token with the write scope may make: one site, named `name`, on a subnet of
/// its own.
async fn change(hub: &Hub, token: &str, name: &str, declared: Option<&str>) {
    static NEXT: AtomicU8 = AtomicU8::new(1);
    let subnet = format!("10.{}.0.0/24", NEXT.fetch_add(1, Ordering::Relaxed));
    let query = "mutation($name: String!, $subnet: Cidr!) { \
        createSite(name: $name, subnet: $subnet) { id } }";
    let variables = json!({ "name": name, "subnet": subnet });
    let answer = post(hub, token, (query, variables), declared).await;
    assert!(answer["data"]["createSite"]["id"].is_string(), "{answer}");
}

/// A WRITE key made for `made_for`, made with the login `token`; returns its secret.
async fn key(hub: &Hub, token: &str, name: &str, made_for: &str) -> String {
    let input = json!({ "name": name, "scope": "WRITE", "madeFor": made_for,
                        "expiresAt": start() + TimeDelta::days(30) });
    let answer = post(hub, token, (CREATE_KEY, json!({ "input": input })), None).await;
    let created = &answer["data"]["createApiKey"];
    assert_eq!(created["key"]["madeFor"], made_for, "{answer}");
    created["secret"].as_str().unwrap().to_owned()
}

/// The audit entries on sites (`kind` only, when given), oldest first.
async fn site_entries(hub: &Hub, token: &str, kind: Option<&str>) -> Vec<Value> {
    let answer = post(hub, token, (AUDIT, json!({ "kind": kind })), None).await;
    let edges = answer["data"]["audit"]["edges"].as_array().unwrap().clone();
    let mut nodes: Vec<Value> = edges
        .into_iter()
        .map(|edge| edge["node"].clone())
        .filter(|node| node["targetType"] == "site")
        .collect();
    nodes.reverse();
    nodes
}

fn who(node: &Value) -> (Value, Value, Value) {
    (
        node["actorKind"].clone(),
        node["tokenKind"].clone(),
        node["apiKeyName"].clone(),
    )
}

#[sqlx::test(migrations = "./migrations")]
async fn log_014_each_request_is_recorded_with_its_kind(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    let user_key = key(&hub, &login, "deploy", "USER").await;
    let agent_key = key(&hub, &login, "claude", "AGENT").await;

    change(&hub, &login, "login", None).await;
    change(&hub, &login, "login-declared", Some("agent")).await;
    change(&hub, &login, "login-other-value", Some("user")).await;
    change(&hub, &agent_key, "agent-key", None).await;
    change(&hub, &agent_key, "agent-key-declared", Some("AGENT")).await;
    change(&hub, &agent_key, "agent-key-says-user", Some("user")).await;
    change(&hub, &user_key, "user-key", None).await;
    change(&hub, &user_key, "user-key-declared", Some("agent")).await;

    let entries = site_entries(&hub, &login, None).await;
    let recorded: Vec<_> = entries.iter().map(who).collect();
    let login_user = (json!("USER"), json!("LOGIN"), Value::Null);
    let login_agent = (json!("AGENT"), json!("LOGIN"), Value::Null);
    let agent = (json!("AGENT"), json!("API_KEY"), json!("claude"));
    assert_eq!(
        recorded,
        [
            login_user.clone(),
            login_agent,
            login_user,
            agent.clone(),
            agent.clone(),
            agent,
            (json!("USER"), json!("API_KEY"), json!("deploy")),
            (json!("AGENT"), json!("API_KEY"), json!("deploy")),
        ]
    );
    assert!(
        entries
            .iter()
            .all(|node| node["operatorName"] == "operator")
    );

    let events = post(&hub, &login, (EVENTS, json!({})), None).await;
    let kinds: Vec<&Value> = events["data"]["events"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| &edge["node"]["actorKind"])
        .collect();
    assert!(kinds.contains(&&json!("AGENT")) && kinds.contains(&&json!("USER")));
    assert!(!kinds.contains(&&Value::Null), "every new event has a kind");
}

#[sqlx::test(migrations = "./migrations")]
async fn log_020_the_audit_list_filters_by_actor_kind(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    change(&hub, &login, "by-user", None).await;
    change(&hub, &login, "by-agent", Some("agent")).await;

    let agents = site_entries(&hub, &login, Some("AGENT")).await;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["actorKind"], "AGENT");
    let users = site_entries(&hub, &login, Some("USER")).await;
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["actorKind"], "USER");
    assert_eq!(
        site_entries(&hub, &login, Some("HUB")).await,
        [] as [Value; 0]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_018_no_entry_holds_a_token_or_a_key(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    let secret = key(&hub, &login, "claude", "AGENT").await;
    change(&hub, &secret, "with-key", Some("agent")).await;
    change(&hub, &login, "with-login", Some("agent")).await;

    let rows: Vec<String> = sqlx::query_scalar(
        "select to_jsonb(a)::text from events.audit_log a \
         union all select to_jsonb(e)::text from events.event_log e",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let body = secret.strip_prefix("anima_").unwrap();
    for row in &rows {
        for needle in [
            secret.as_str(),
            body,
            &body[..12],
            login.as_str(),
            &login[..12],
        ] {
            assert!(!row.contains(needle), "an entry holds a token: {row}");
        }
    }
    let names: Vec<Option<String>> =
        sqlx::query_scalar("select api_key_name from events.audit_log where target_type = 'site'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(names.contains(&Some("claude".to_owned())));
}

#[sqlx::test(migrations = "./migrations")]
async fn log_006_an_entry_from_before_the_actor_kinds_reads_back_with_none(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    let mut conn = pool.acquire().await.unwrap();
    let at = start() - TimeDelta::days(3);
    for parent in ["audit_log", "event_log"] {
        sqlx::query("select events.ensure_day_partition($1, $2)")
            .bind(parent)
            .bind(at)
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    sqlx::query(
        "insert into events.audit_log (id, at, operator_id, operator_name, action, target_type, \
         target_id, client_addr, change) values (gen_random_uuid(), $1, gen_random_uuid(), \
         'operator', 'create', 'site', 'old', '100.64.0.7', '{}')",
    )
    .bind(at)
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query(
        "insert into events.event_log (id, at, severity, source, message) \
         values (gen_random_uuid(), $1, 'info', 'hub', 'An old event.')",
    )
    .bind(at)
    .execute(&mut *conn)
    .await
    .unwrap();
    drop(conn);

    let old = site_entries(&hub, &login, None).await;
    assert_eq!(who(&old[0]), (Value::Null, Value::Null, Value::Null));
    for kind in ["USER", "AGENT", "HUB"] {
        assert_eq!(
            site_entries(&hub, &login, Some(kind)).await,
            [] as [Value; 0]
        );
    }
    let events = post(&hub, &login, (EVENTS, json!({})), None).await;
    let old_event = events["data"]["events"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| edge["node"]["message"] == "An old event.")
        .unwrap();
    assert_eq!(old_event["node"]["actorKind"], Value::Null);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_015_made_for_cannot_change(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    key(&hub, &login, "deploy", "USER").await;
    let changed =
        sqlx::query("update access.api_tokens set made_for = 'agent' where kind = 'api_key'")
            .execute(&pool)
            .await;
    assert!(
        changed.is_err(),
        "the database refuses a change of made_for"
    );
}

/// The actor kind a WebSocket's `connection_init` `payload` signs in as.
async fn socket_kind(pool: &PgPool, payload: Value) -> ActorKind {
    let identity = access::identify_connection(pool, &payload, start())
        .await
        .unwrap()
        .expect("an identity");
    identity.operator().unwrap().actor.kind()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_016_a_socket_declares_in_its_connection_init(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    let agent_key = key(&hub, &login, "claude", "AGENT").await;
    let bearer = |token: &str| format!("Bearer {token}");

    let plain = json!({ "authorization": bearer(&login) });
    assert_eq!(socket_kind(&pool, plain).await, ActorKind::User);
    let declared = json!({ "authorization": bearer(&login), "actor": "agent" });
    assert_eq!(socket_kind(&pool, declared).await, ActorKind::Agent);
    let other = json!({ "authorization": bearer(&login), "actor": "hub" });
    assert_eq!(socket_kind(&pool, other).await, ActorKind::User);
    let lowered = json!({ "authorization": bearer(&agent_key), "actor": "user" });
    assert_eq!(socket_kind(&pool, lowered).await, ActorKind::Agent);
}
