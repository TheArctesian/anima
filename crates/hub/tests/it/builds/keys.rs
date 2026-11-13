//! An API key never enrols or revokes a builder, publishes or cancels a release, or claims a job:
//! a builder token outlives the key that would have enrolled it (D-23, R-100, R-107). A builder
//! token reaches no root field but the builder mutations, by fragment, introspection or socket
//! (BLD-022). Only a builder token gets upload URLs (R-108).

use std::sync::Arc;

use anima_hub::clock::Clock;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use super::harness::{COMMIT, CREATE, LINUX, claim, create_job, enrol, operator};
use crate::{
    events::client::{Received, TestClock, ack, bearer, init, open, receive, send, serve_router},
    hub::{Hub, error, start},
};

/// An API key of the operator with `scope`, made of `fill`, as `createApiKey` stores one.
async fn api_key(pool: &PgPool, scope: &str, fill: u8) -> String {
    let secret = [fill; 32];
    let name = format!("script-{scope}");
    sqlx::query(
        "INSERT INTO access.api_tokens \
         (id, operator_id, token_hash, kind, made_for, name, display_prefix, scope, expires_at, created_at) \
         SELECT gen_random_uuid(), id, $1, 'api_key', 'user', $4, 'anima_BwcHBw', $5, $2, $3 \
         FROM access.operators",
    )
    .bind(Sha256::digest(secret).to_vec())
    .bind(start() + chrono::TimeDelta::days(30))
    .bind(start())
    .bind(name)
    .bind(scope)
    .execute(pool)
    .await
    .unwrap();
    format!("anima_{}", URL_SAFE_NO_PAD.encode(secret))
}

/// A WRITE API key of the operator.
async fn write_key(pool: &PgPool) -> String {
    api_key(pool, "write", 7).await
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

fn forbidden(response: &Value, what: &str) {
    assert_eq!(error(response).1, "FORBIDDEN", "{what}: {response}");
    assert!(response["data"].is_null(), "{what}: {response}");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_102_a_write_api_key_may_not_enrol_revoke_publish_or_cancel(pool: PgPool) {
    let operator = operator(&pool).await;
    let key = write_key(&pool).await;
    let hub = Hub::new(&pool, start());
    let job = create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    let enrol =
        format!("mutation {{ enrolBuilder(name: \"script\", platform: {LINUX}) {{ token }} }}");
    forbidden(
        &hub.post(Some(&key), &enrol, json!({})).await,
        "enrolBuilder",
    );
    let revoke = "mutation($id: ID!) { revokeBuilder(id: $id) }";
    let revoked = hub
        .post(Some(&key), revoke, json!({ "id": builder.id }))
        .await;
    forbidden(&revoked, "revokeBuilder");
    let variables = json!({ "version": "0.1.13", "commit": COMMIT });
    forbidden(
        &hub.post(Some(&key), CREATE, variables).await,
        "createRelease",
    );
    let cancel = "mutation($id: ID!) { cancelBuildJob(id: $id) { id } }";
    let cancelled = hub.post(Some(&key), cancel, json!({ "id": job })).await;
    forbidden(&cancelled, "cancelBuildJob");
    assert_eq!(count(&pool, "builds.builders").await, 1);
    assert_eq!(count(&pool, "builds.releases").await, 1);
    let state: String = sqlx::query_scalar("SELECT state FROM builds.jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "queued");
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_022_a_write_api_key_may_not_claim_a_job(pool: PgPool) {
    let operator = operator(&pool).await;
    let key = write_key(&pool).await;
    let hub = Hub::new(&pool, start());
    create_job(&hub, &operator, "0.1.12").await;
    forbidden(&claim(&hub, &key).await, "claimBuildJob");
}

/// R-108: only a builder token gets upload URLs; an API key of either scope and an operator
/// token are refused before any resolver, so no URL is signed for them.
#[sqlx::test(migrations = "./migrations")]
async fn r_108_only_a_builder_token_may_get_upload_urls(pool: PgPool) {
    let operator = operator(&pool).await;
    let key = write_key(&pool).await;
    let read = api_key(&pool, "read", 9).await;
    let hub = Hub::new(&pool, start());
    let job = create_job(&hub, &operator, "0.1.12").await;
    let urls = "mutation($id: ID!) { buildUploadUrls(id: $id, files: []) { url } }";
    for token in [&key, &read, &operator] {
        let response = hub.post(Some(token), urls, json!({ "id": job })).await;
        forbidden(&response, "buildUploadUrls");
    }
    let aliased = "mutation($id: ID!) { a: buildUploadUrls(id: $id, files: []) { url } }";
    let response = hub.post(Some(&key), aliased, json!({ "id": job })).await;
    forbidden(&response, "aliased buildUploadUrls");
    assert_eq!(count(&pool, "builds.job_files").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_022_a_builder_token_reaches_no_root_by_fragment_or_introspection(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let builder = enrol(&hub, &operator, "pop-os").await;
    for query in [
        "{ ... on Query { builders { id } } }",
        "query { ...F } fragment F on Query { me { id } }",
        "mutation { ... on Mutation { logout } }",
        "{ __schema { types { name } } }",
        "{ __type(name: \"Query\") { name } }",
        "{ x: me { id } }",
    ] {
        forbidden(
            &hub.post(Some(&builder.token), query, json!({})).await,
            query,
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_022_a_builder_token_reaches_nothing_over_the_socket(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let builder = enrol(&hub, &operator, "pop-os").await;
    let clock: Arc<dyn Clock> = Arc::new(TestClock::at(start()));
    let served = serve_router(&pool, clock).await;
    for query in [
        "subscription { eventAdded(minSeverity: INFO) { message } }",
        "subscription { buildJobUpdated { job { id } } }",
        "query { me { id } }",
        "mutation { logout }",
    ] {
        let mut socket = open(served.addr).await;
        assert_eq!(init(&mut socket, bearer(&builder.token)).await, ack());
        let subscribe = json!({ "id": "1", "type": "subscribe", "payload": { "query": query } });
        send(&mut socket, &subscribe).await;
        let Received::Json(message) = receive(&mut socket).await else {
            panic!("{query}: the socket closed");
        };
        let payload = &message["payload"];
        assert_eq!(
            payload["errors"][0]["extensions"]["code"], "FORBIDDEN",
            "{query}: {message}"
        );
        assert!(payload["data"].is_null(), "{query}: {message}");
    }
}
