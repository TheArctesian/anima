//! PRJ-039, PRJ-041, LOG-012: the attempt's API: a start, a replaced start, the one-time link,
//! the refusal of a declared agent, a cancel, the audit entries and `githubConnectChanged`.

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Actor, Bus, ClientAddr, Operator},
    },
    ids::OperatorId,
    schema::{Dependencies, build_schema},
};
use async_graphql::{Request, futures_util::StreamExt as _};
use axum::{
    body::Body,
    http::{Request as HttpRequest, header},
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{CANCEL, OPEN, STATE, link, start_with, started};
use crate::{
    hub::{dependencies, error, start},
    sources::{Kit, kit},
};

pub(super) const PAGE: &str = "https://hub.anima.ing/connect/github";

/// The audit entries of the flow before an account exists: `(action, actor kind, change)`.
pub(super) async fn audited(pool: &PgPool) -> Vec<(String, Option<String>, Value)> {
    sqlx::query_as(
        "SELECT action, actor_kind, change FROM events.audit_log \
         WHERE target_type = 'setting' AND target_id = 'sources.github.connect' ORDER BY at, action",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Posts `query` with the agent declaration (LOG-016).
async fn as_agent(kit: &Kit, query: &str, variables: Value) -> Value {
    let body = json!({ "query": query, "variables": variables }).to_string();
    let request = HttpRequest::post("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", kit.token))
        .header("anima-actor", "agent")
        .body(Body::from(body))
        .unwrap();
    let (status, text) = kit.hub.send(request).await;
    assert!(status.is_success(), "{status} {text}");
    serde_json::from_str(&text).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_start_shows_an_attempt_that_waits_for_the_browser(pool: PgPool) {
    let kit = kit(&pool).await;
    let response = start_with(&kit, json!({ "label": " Mine ", "organisation": null })).await;
    let attempt = &response["data"]["startGithubConnect"];
    assert_eq!(attempt["phase"], "WAITING_FOR_BROWSER", "{response}");
    assert_eq!(attempt["startedBy"], "USER");
    assert_eq!(attempt["pageUrl"], PAGE, "no value in the page link");
    assert_eq!(attempt["message"], "Opening GitHub in your browser.");
    let started_at: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(attempt["startedAt"].clone()).unwrap();
    let expires_at: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(attempt["expiresAt"].clone()).unwrap();
    assert_eq!(expires_at - started_at, TimeDelta::minutes(55));
    for field in [
        "appId",
        "appSlug",
        "ownerLogin",
        "installUrl",
        "appSettingsUrl",
        "failure",
        "account",
    ] {
        assert_eq!(attempt[field], Value::Null, "{field}");
    }
    let shown = kit.hub.post(Some(&kit.token), STATE, json!({})).await;
    assert_eq!(shown["data"]["githubConnect"]["id"], attempt["id"]);
    let entries = audited(&pool).await;
    assert_eq!(entries.len(), 1);
    let (action, actor, change) = &entries[0];
    assert_eq!((action.as_str(), actor.as_deref()), ("start", Some("user")));
    assert_eq!(
        change,
        &json!({ "label": "Mine", "organisation": null, "actor": "user" })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_start_checks_the_label_and_the_organisation(pool: PgPool) {
    let kit = kit(&pool).await;
    for input in [
        json!({ "label": "  " }),
        json!({ "label": "x".repeat(65) }),
        json!({ "organisation": "not--a-login" }),
        json!({ "organisation": "a/b" }),
    ] {
        let response = start_with(&kit, input.clone()).await;
        assert_eq!(error(&response).1, "VALIDATION", "{input}");
    }
    let shown = kit.hub.post(Some(&kit.token), STATE, json!({})).await;
    assert_eq!(
        shown["data"]["githubConnect"],
        Value::Null,
        "nothing started"
    );
    assert_eq!(audited(&pool).await, Vec::new(), "nothing audited");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_second_start_replaces_an_attempt_that_made_no_app(pool: PgPool) {
    let kit = kit(&pool).await;
    let first = started(&kit).await;
    let second = started(&kit).await;
    assert_ne!(first, second);
    let shown = kit.hub.post(Some(&kit.token), STATE, json!({})).await;
    assert_eq!(shown["data"]["githubConnect"]["id"], second.as_str());
    let refused = kit
        .hub
        .post(Some(&kit.token), OPEN, json!({ "id": first }))
        .await;
    assert_eq!(error(&refused).1, "NOT_FOUND");
    let refused = kit
        .hub
        .post(Some(&kit.token), CANCEL, json!({ "id": "not-an-id" }))
        .await;
    assert_eq!(error(&refused).1, "NOT_FOUND");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_the_link_is_one_time_and_a_new_one_differs(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = started(&kit).await;
    let response = kit
        .hub
        .post(Some(&kit.token), OPEN, json!({ "id": id }))
        .await;
    let opened = &response["data"]["openGithubConnect"];
    let url = opened["url"].as_str().unwrap();
    let ticket = url
        .strip_prefix(&format!("{PAGE}?ticket="))
        .expect("the page with a ticket");
    assert_eq!(ticket.len(), 43);
    let expires_at: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(opened["expiresAt"].clone()).unwrap();
    assert_eq!(expires_at, start() + TimeDelta::minutes(5));
    assert_ne!(link(&kit, &id).await, url, "a new ticket each time");
    let shown = kit.hub.post_text(Some(&kit.token), STATE, json!({})).await;
    assert!(!shown.contains(ticket), "no answer but the link holds it");
    let entries = audited(&pool).await;
    assert!(
        entries
            .iter()
            .all(|(_, _, change)| !change.to_string().contains(ticket))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_012_prj_041_a_declared_agent_may_start_but_not_open_the_link(pool: PgPool) {
    let kit = kit(&pool).await;
    let start_document = super::START;
    let response = as_agent(&kit, start_document, json!({ "input": null })).await;
    let attempt = &response["data"]["startGithubConnect"];
    assert_eq!(attempt["startedBy"], "AGENT", "{response}");
    let id = attempt["id"].as_str().unwrap();
    let refused = as_agent(&kit, OPEN, json!({ "id": id })).await;
    assert_eq!(error(&refused).1, "FORBIDDEN");
    assert_eq!(refused["data"], Value::Null);
    assert!(!refused.to_string().contains("ticket="));
    assert!(link(&kit, id).await.contains("ticket="), "the operator may");
    let entries = audited(&pool).await;
    assert_eq!(entries[0].1.as_deref(), Some("agent"));
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_cancel_ends_the_attempt_and_is_audited(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = started(&kit).await;
    let response = kit
        .hub
        .post(Some(&kit.token), CANCEL, json!({ "id": id }))
        .await;
    let cancelled = &response["data"]["cancelGithubConnect"];
    assert_eq!(
        (cancelled["phase"].as_str(), cancelled["message"].as_str()),
        (Some("CANCELLED"), Some("Cancelled."))
    );
    let again = kit
        .hub
        .post(Some(&kit.token), CANCEL, json!({ "id": id }))
        .await;
    assert_eq!(again["data"]["cancelGithubConnect"]["phase"], "CANCELLED");
    let refused = kit
        .hub
        .post(Some(&kit.token), OPEN, json!({ "id": id }))
        .await;
    assert_eq!(error(&refused).1, "CONFLICT");
    let actions: Vec<String> = audited(&pool)
        .await
        .into_iter()
        .map(|(a, _, _)| a)
        .collect();
    // The clock is stopped, so the entries come by action.
    assert_eq!(actions, ["cancel", "start"], "one cancel entry");
    let shown = kit.hub.post(Some(&kit.token), STATE, json!({})).await;
    assert_eq!(shown["data"]["githubConnect"]["phase"], "CANCELLED");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_github_connect_changed_sends_each_change(pool: PgPool) {
    let kit = kit(&pool).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let schema = build_schema(Dependencies {
        sources: kit.reach.clone(),
        ..dependencies(&pool, clock, Bus::new(16))
    });
    let operator = Operator {
        id: "0190a000-0000-7000-8000-000000000001"
            .parse::<OperatorId>()
            .unwrap(),
        name: "operator".to_owned(),
        actor: Actor::for_tests(),
    };
    let query = "subscription { githubConnectChanged { id phase message pageUrl } }";
    let request = Identity::for_tests(operator, start() + TimeDelta::days(3650))
        .attach(Request::new(query))
        .data(ClientAddr("100.64.0.7".parse().unwrap()));
    let mut stream = schema.execute_stream(request);
    let first = tokio::time::timeout(Duration::from_millis(100), stream.next()).await;
    assert!(first.is_err(), "nothing was published yet");

    let one = started(&kit).await;
    let _ = link(&kit, &one).await;
    let two = started(&kit).await;
    kit.hub
        .post(Some(&kit.token), CANCEL, json!({ "id": two }))
        .await;
    let mut sent = Vec::new();
    while let Ok(Some(next)) = tokio::time::timeout(Duration::from_millis(300), stream.next()).await
    {
        assert!(next.errors.is_empty(), "{:?}", next.errors);
        let changed = next.data.into_json().unwrap()["githubConnectChanged"].clone();
        sent.push((
            changed["id"].as_str().unwrap().to_owned(),
            changed["phase"].as_str().unwrap().to_owned(),
        ));
    }
    assert_eq!(
        sent,
        [
            (one, "WAITING_FOR_BROWSER".to_owned()),
            (two.clone(), "WAITING_FOR_BROWSER".to_owned()),
            (two, "CANCELLED".to_owned()),
        ],
        "a new link changes nothing that is shown"
    );
}
