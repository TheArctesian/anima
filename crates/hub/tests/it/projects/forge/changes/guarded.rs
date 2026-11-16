//! D-23, R-100, K-E1 Section 13.5: the run controls, a start and a commit status are login or
//! browser token only. A read key and a write key are refused and send nothing; a login token,
//! also for a request that declares an agent, is recorded with who it was (LOG-014 to LOG-018).
//! LOG-007: a workflow input value and GitHub's reply reach no log line, error message, audit
//! entry or event.

use anima_hub::fleet::events::process_log;
use axum::{
    body::Body,
    http::{Request, header},
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    answer, audits,
    controls::{asked, control, run_in},
    starts::{DISPATCH, STATUS, accepted, workflows},
};
use crate::{
    hub::{error, start},
    logs::Capture,
    projects::forge::{BASE, Forge, forge, sha},
};

const KIND_REFUSED: &str = "This token cannot do that. Sign in to the hub to do it.";

/// The secret of a key of `scope` made by the signed-in operator for a user.
async fn key(forge: &Forge, scope: &str) -> String {
    let query = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let input = json!({ "input": { "name": format!("{scope}-key"), "scope": scope,
        "madeFor": "USER", "expiresAt": start() + TimeDelta::days(30) } });
    let response = forge.kit.post(query, input).await;
    response["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("no key: {response}"))
        .to_owned()
}

/// GitHub accepts each of the five changes.
fn script(forge: &Forge) {
    forge.ok(
        "/actions/runs/5",
        &run_in(5, "completed", Some("failure"), 1),
    );
    forge.ok("/actions/runs/6", &run_in(6, "in_progress", None, 1));
    answer(forge, ("POST", "/actions/runs/5/rerun"), 201, &json!({}));
    answer(
        forge,
        ("POST", "/actions/runs/5/rerun-failed-jobs"),
        201,
        &json!({}),
    );
    answer(forge, ("POST", "/actions/runs/6/cancel"), 202, &json!({}));
    workflows(forge);
    accepted(forge);
    answer(
        forge,
        ("POST", &format!("/statuses/{}", sha(9))),
        201,
        &json!({}),
    );
}

/// The five roots, each with its document and variables that work.
fn calls(forge: &Forge) -> Vec<(&'static str, String, Value)> {
    let with = |mut variables: Value| {
        variables["repository"] = json!(forge.repository);
        variables
    };
    vec![
        (
            "rerunWorkflowRun",
            control("rerunWorkflowRun"),
            with(asked(5)),
        ),
        (
            "rerunFailedWorkflowJobs",
            control("rerunFailedWorkflowJobs"),
            with(asked(5)),
        ),
        (
            "cancelWorkflowRun",
            control("cancelWorkflowRun"),
            with(asked(6)),
        ),
        (
            "dispatchWorkflow",
            DISPATCH.to_owned(),
            with(json!({ "workflowId": "31", "branch": "main" })),
        ),
        (
            "setCommitStatus",
            STATUS.to_owned(),
            with(json!({ "sha": sha(9), "state": "SUCCESS", "context": "anima/build" })),
        ),
    ]
}

/// Changes sent below the repository so far.
fn changes_sent(forge: &Forge) -> usize {
    forge.kit.stub.with(|s| {
        s.heard
            .iter()
            .filter(|h| h.method != "GET" && h.path.starts_with(BASE))
            .count()
    })
}

/// Posts `query` with `token` and the agent declaration (LOG-016).
async fn as_agent(forge: &Forge, token: &str, query: &str, variables: Value) -> Value {
    let body = json!({ "query": query, "variables": variables }).to_string();
    let request = Request::post("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("anima-actor", "agent")
        .body(Body::from(body))
        .unwrap();
    let (status, text) = forge.kit.hub.send(request).await;
    assert!(status.is_success(), "{status} {text}");
    serde_json::from_str(&text).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_the_five_roots_refuse_every_api_key_and_take_a_login_token(pool: PgPool) {
    let forge = forge(&pool).await;
    script(&forge);
    for scope in ["READ", "WRITE"] {
        let token = key(&forge, scope).await;
        for (root, query, variables) in calls(&forge) {
            let refused = forge.kit.hub.post(Some(&token), &query, variables).await;
            assert_eq!(
                error(&refused),
                (KIND_REFUSED.to_owned(), "FORBIDDEN".to_owned()),
                "{root} {scope}"
            );
        }
    }
    assert_eq!(changes_sent(&forge), 0, "a key sends nothing");
    assert!(audits(&pool).await.is_empty(), "a key records nothing");

    for (root, query, variables) in calls(&forge) {
        forge.clock.advance(2);
        let response = forge
            .kit
            .hub
            .post(Some(&forge.kit.token), &query, variables)
            .await;
        assert!(response["errors"].is_null(), "{root}: {response}");
        let entries = audits(&pool).await;
        let last = entries.last().unwrap();
        assert_eq!(
            (&last["actor_kind"], &last["token_kind"]),
            (&json!("user"), &json!("login"))
        );
    }
    for (root, query, variables) in calls(&forge) {
        forge.clock.advance(2);
        let response = as_agent(&forge, &forge.kit.token, &query, variables).await;
        assert!(response["errors"].is_null(), "{root} declared: {response}");
        let entries = audits(&pool).await;
        assert_eq!(entries.last().unwrap()["actor_kind"], "agent", "{root}");
    }
    assert_eq!(changes_sent(&forge), 10);
}

const VALUE: &str = "INPUT-VALUE-SECRET-9035";
const ANSWERED: &str = "GITHUB-ANSWERED-6402";

#[sqlx::test(migrations = "./migrations")]
async fn log_007_a_workflow_input_value_and_githubs_answer_are_never_kept(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let forge = forge(&pool).await;
    workflows(&forge);
    let inputs = json!([{ "name": "token", "value": VALUE }]);
    let start =
        |branch: &str| json!({ "workflowId": "ci.yml", "branch": branch, "inputs": inputs });
    accepted(&forge);
    let made = forge.ask(DISPATCH, start("main")).await;
    assert!(made["errors"].is_null(), "{made}");
    let refusal = json!({ "message": ANSWERED, "errors": [{ "value": VALUE }] });
    answer(
        &forge,
        ("POST", "/actions/workflows/31/dispatches"),
        422,
        &refusal,
    );
    forge.clock.advance(2);
    let refused = forge.ask(DISPATCH, start("main")).await;
    assert_eq!(error(&refused).1, "VALIDATION", "{refused}");
    answer(
        &forge,
        ("POST", "/actions/workflows/31/dispatches"),
        500,
        &refusal,
    );
    forge.clock.advance(2);
    let unknown = forge.ask(DISPATCH, start("main")).await;
    assert_eq!(error(&unknown).1, "UNAVAILABLE", "{unknown}");
    let invalid = forge.ask(DISPATCH, start("bad..branch")).await;
    assert_eq!(error(&invalid).1, "VALIDATION", "{invalid}");
    for response in [&refused, &unknown, &invalid] {
        let text = response.to_string();
        assert!(!text.contains(VALUE) && !text.contains(ANSWERED), "{text}");
    }

    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 3, "{entries:?}");
    assert!(entries.iter().all(|entry| entry["change"]["inputs"] == 1));
    let events: Vec<String> =
        sqlx::query_scalar("SELECT to_jsonb(e)::text FROM events.event_log e")
            .fetch_all(&pool)
            .await
            .unwrap();
    let kept = format!("{entries:?} {events:?}");
    assert!(!kept.contains(VALUE), "an input value was kept");
    assert!(!kept.contains(ANSWERED), "GitHub's answer was kept");
    let log = capture.text();
    assert!(!log.is_empty(), "the log was captured");
    assert!(!log.contains(VALUE), "an input value is in the log");
    assert!(!log.contains(ANSWERED), "GitHub's answer is in the log");
}
