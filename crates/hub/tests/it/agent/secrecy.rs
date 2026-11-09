//! MET-012, MET-013, LOG-007: the hub keeps only the SHA-256 of a code's or a token's random
//! bytes, and no code or token reaches the process log, an event or an audit entry. The whole
//! flow, a push included, runs as the runtime role (K2 "Roles").

use std::{
    io,
    sync::{Arc, Mutex},
};

use anima_hub::events::process_log;
use axum::http::StatusCode;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};

use super::{
    batches::{cpu_batch, samples_of},
    harness::{Agents, PVE_01, pushing_now},
};

/// The SHA-256 of the random bytes behind `secret` (its text after the prefix).
fn hash_of(secret: &str, prefix: &str) -> Vec<u8> {
    let bytes = URL_SAFE_NO_PAD
        .decode(secret.strip_prefix(prefix).unwrap())
        .unwrap();
    Sha256::digest(bytes).to_vec()
}

/// Every stored value of the two agent tables and the logs, as text.
async fn stored_text(pool: &PgPool) -> String {
    let sql = "SELECT concat_ws(' ', \
         (SELECT string_agg(a::text, ' ') FROM metrics.agents a), \
         (SELECT string_agg(e::text, ' ') FROM metrics.enrolments e), \
         (SELECT string_agg(l::text, ' ') FROM events.event_log l), \
         (SELECT string_agg(l::text, ' ') FROM events.audit_log l))";
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_met_013_only_the_hashes_of_codes_and_tokens_are_stored(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let code = agents.code_for(PVE_01).await;
    let stored_code: Vec<u8> =
        sqlx::query_scalar("SELECT code_sha256 FROM metrics.enrolments WHERE used_at IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored_code, hash_of(&code, "anima_enrol_"));

    let token = agents.enrolled(&code).await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let next = agents.renewed(&token).await;
    let (current, renewed): (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT token_sha256, next_token_sha256 FROM metrics.agents WHERE device_id = $1::uuid",
    )
    .bind(PVE_01)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(current, hash_of(&token, "anima_agent_"));
    assert_eq!(renewed, hash_of(&next, "anima_agent_"));

    let everything = stored_text(&pool).await;
    for secret in [&code, &token, &next] {
        let body = &secret[12..];
        assert!(
            !everything.contains(body),
            "a secret is stored in the clear"
        );
    }
    assert!(!everything.contains("anima_"), "{everything}");
}

/// Collects everything the process log writes.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_007_the_process_log_of_the_whole_flow_holds_no_code_or_token(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in wiring::uri_log: a second dispatcher keeps another test thread from caching this
    // test's callsites as disabled.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let code = agents.code_for(PVE_01).await;
    let token = agents.enrolled(&code).await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(agents.enrol(&code).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        agents.push(&token, &cpu_batch(PVE_01, now, 5.0)).await.0,
        StatusCode::NO_CONTENT
    );
    // A body that is not a batch, quoting the token: its parse error is never logged.
    let quoted = json!({ "device": PVE_01, "series": token, "points": [] }).to_string();
    agents
        .push_raw(Some(&format!("Bearer {token}")), quoted, "100.64.0.11")
        .await;
    let next = agents.renewed(&token).await;
    agents.renewed(&next).await;
    agents.renew(&token).await;
    agents
        .renew_raw(Some(&format!("Bearer {next}")), "[1]", "100.64.0.9")
        .await;
    agents
        .enrol_body(
            json!({ "code": format!("{code}x") }).to_string(),
            "100.64.0.10",
        )
        .await;
    let query = "mutation($id: ID!) { revokeAgentToken(deviceId: $id) { id } }";
    agents.operate(query, json!({ "id": PVE_01 })).await;
    agents.renew(&next).await;

    let logged = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("/agent/v1/enrol"),
        "the capture saw the flow"
    );
    assert!(
        logged.contains("/agent/v1/renew"),
        "the capture saw the flow"
    );
    assert!(
        logged.contains("/agent/v1/push"),
        "the capture saw the flow"
    );
    assert!(
        !logged.contains("anima_agent_"),
        "a token reached the process log"
    );
    assert!(
        !logged.contains("anima_enrol_"),
        "a code reached the process log"
    );
    for secret in [&code, &token, &next] {
        assert!(
            !logged.contains(&secret[12..]),
            "a secret reached the process log"
        );
    }
}

/// A pool whose every connection runs as `anima_runtime`.
async fn runtime_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(4)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE anima_runtime").execute(conn).await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k2_roles_the_runtime_role_runs_the_whole_agent_flow(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&runtime, now).await;
    let token = agents.token_for(PVE_01).await;
    let pushed = agents.push(&token, &cpu_batch(PVE_01, now, 5.0)).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT, "{}", pushed.1);
    assert_eq!(samples_of(&pool, PVE_01).await, 1);
    let next = agents.renewed(&token).await;
    agents.renewed(&next).await;
    let query = "mutation($id: ID!) { revokeAgentToken(deviceId: $id) { id } }";
    let revoked = agents.operate(query, json!({ "id": PVE_01 })).await;
    assert_eq!(
        revoked["data"]["revokeAgentToken"]["id"], PVE_01,
        "{revoked}"
    );
    assert_eq!(agents.renew(&next).await.0, StatusCode::UNAUTHORIZED);
}
